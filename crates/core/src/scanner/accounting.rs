use super::{
    Coverage, DecimalQuantity, EntryKind, ObjectIdentity, ReadDecision, ScanEntry, ScanErrorCode,
    ScanFailure, ScanLimits, ScanMetrics, ScanReadPolicy, StopReason,
};
use crate::workspaces::WorkspaceId;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregateMetrics {
    pub metrics: ScanMetrics,
    pub coverage: Coverage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceAccounting {
    pub metrics: ScanMetrics,
    pub coverage: Coverage,
    pub covered_by_workspace_ids: Vec<WorkspaceId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationOutcome {
    File,
    Directory,
    Unmeasured,
    Duplicate {
        owner: WorkspaceId,
        directory_coverage: Option<Coverage>,
    },
}

// Native identity is 40 bytes; ownership and bounded dependent-root bits fit in
// the same fixed slot. State 0 is empty, 1 file, 2 pending directory, 3 complete
// directory, 4 partial directory, 5 unavailable directory, 6 unknown-allocation file.
#[derive(Clone, Copy)]
#[repr(C)]
struct IdentitySlot {
    identity: ObjectIdentity,
    owner: i64,
    dependents: u32,
    state: u8,
    padding: [u8; 11],
}

impl IdentitySlot {
    const EMPTY: Self = Self {
        identity: ObjectIdentity {
            volume_uuid: [0; 16],
            file_id: 0,
            birth_seconds: 0,
            birth_nanoseconds: 0,
        },
        owner: 0,
        dependents: 0,
        state: 0,
        padding: [0; 11],
    };

    fn directory_coverage(self) -> Option<Coverage> {
        match self.state {
            2 | 4 => Some(Coverage::Partial),
            3 => Some(Coverage::CompleteWithinPolicy),
            5 => Some(Coverage::Unavailable),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Default)]
struct Counters {
    logical: u64,
    allocated: u64,
    files: u64,
    directories: u64,
    references: u64,
    measured: u64,
    unknown: u64,
    excluded: u64,
    unmeasured: u64,
    pending_directories: u64,
    incomplete: bool,
    allocation_incomplete: bool,
}

fn add(value: &mut u64, amount: u64) -> Result<(), StopReason> {
    *value = value
        .checked_add(amount)
        .ok_or(StopReason::QuantityOverflow)?;
    Ok(())
}

impl Counters {
    fn observe(&mut self, metadata: &ScanEntry) -> Result<(), StopReason> {
        if metadata.facts.kind == EntryKind::Directory {
            add(&mut self.directories, 1)?;
            return add(&mut self.pending_directories, 1);
        }
        add(&mut self.files, 1)?;
        add(&mut self.logical, metadata.logical_bytes.unwrap())?;
        if let Some(bytes) = metadata.allocated_bytes {
            add(&mut self.allocated, bytes)?;
            add(&mut self.measured, 1)?;
        } else {
            add(&mut self.unknown, 1)?;
        }
        Ok(())
    }

    fn snapshot(self) -> AggregateMetrics {
        let incomplete = self.incomplete || self.pending_directories > 0;
        let logical_coverage = if incomplete {
            Coverage::Partial
        } else {
            Coverage::CompleteWithinPolicy
        };
        let allocated_coverage = if self.measured == 0
            && (self.unknown > 0 || incomplete || self.allocation_incomplete)
        {
            Coverage::Unavailable
        } else if incomplete || self.unknown > 0 || self.allocation_incomplete {
            Coverage::Partial
        } else {
            Coverage::CompleteWithinPolicy
        };
        AggregateMetrics {
            coverage: if logical_coverage == Coverage::CompleteWithinPolicy
                && allocated_coverage == Coverage::CompleteWithinPolicy
            {
                Coverage::CompleteWithinPolicy
            } else {
                Coverage::Partial
            },
            metrics: ScanMetrics {
                observed_logical_bytes: DecimalQuantity(self.logical),
                observed_allocated_bytes: if allocated_coverage == Coverage::Unavailable {
                    None
                } else {
                    Some(DecimalQuantity(self.allocated))
                },
                unique_files: DecimalQuantity(self.files),
                unique_directories: DecimalQuantity(self.directories),
                references_deduplicated: DecimalQuantity(self.references),
                allocation_measured_files: DecimalQuantity(self.measured),
                allocation_unknown_files: DecimalQuantity(self.unknown),
                policy_excluded_entries: DecimalQuantity(self.excluded),
                unmeasured_entries_lower_bound: DecimalQuantity(self.unmeasured),
                unmeasured_bytes: if incomplete {
                    None
                } else {
                    Some(DecimalQuantity(0))
                },
                logical_coverage,
                allocated_coverage,
            },
        }
    }
}

#[derive(Clone, Copy, Default)]
struct RootCounters {
    owner: i64,
    counters: Counters,
    covered_by: u32,
}

pub struct ScanAccounting {
    slots: Vec<IdentitySlot>,
    max_records: u64,
    records: u64,
    roots: [RootCounters; 32],
    global: Counters,
}

impl ScanAccounting {
    pub fn new(limits: ScanLimits) -> Result<Self, ScanFailure> {
        Self::allocate(limits.identity_slots(), limits.max_identity_records())
    }

    fn allocate(slots: usize, max_records: u64) -> Result<Self, ScanFailure> {
        let mut table = Vec::new();
        table.try_reserve_exact(slots).map_err(|_| ScanFailure {
            code: ScanErrorCode::ResourceUnavailable,
        })?;
        table.resize(slots, IdentitySlot::EMPTY);
        Ok(Self {
            slots: table,
            max_records,
            records: 0,
            roots: [RootCounters::default(); 32],
            global: Counters::default(),
        })
    }

    #[cfg(test)]
    fn with_capacity(slots: usize, records: u64) -> Result<Self, ScanFailure> {
        Self::allocate(slots, records)
    }

    fn root_index(&self, owner: WorkspaceId) -> Result<usize, StopReason> {
        self.roots
            .iter()
            .position(|r| r.owner == owner.get())
            .or_else(|| self.roots.iter().position(|r| r.owner == 0))
            .ok_or(StopReason::ResultLimit)
    }

    /// Register empty roots too. The engine supplies roots in sorted numeric ID order.
    pub fn register_workspace(&mut self, owner: WorkspaceId) -> Result<(), StopReason> {
        let index = self.root_index(owner)?;
        self.roots[index].owner = owner.get();
        Ok(())
    }

    fn locate(&self, identity: ObjectIdentity) -> Result<usize, StopReason> {
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        identity.hash(&mut hash);
        let start = hash.finish() as usize % self.slots.len();
        for offset in 0..self.slots.len() {
            let index = (start + offset) % self.slots.len();
            if self.slots[index].state == 0 || self.slots[index].identity == identity {
                return Ok(index);
            }
        }
        Err(StopReason::IdentityLimit)
    }

    /// Policy exclusions are observable counters, never identities or file bytes.
    pub fn record_exclusion(&mut self, owner: WorkspaceId) -> Result<(), StopReason> {
        self.record_omission(owner, true, false, 0)
    }

    fn record_omission(
        &mut self,
        owner: WorkspaceId,
        excluded: bool,
        incomplete: bool,
        lower_bound: u64,
    ) -> Result<(), StopReason> {
        let index = self.root_index(owner)?;
        let mut root = self.roots[index];
        let mut global = self.global;
        if excluded {
            add(&mut root.counters.excluded, 1)?;
            add(&mut global.excluded, 1)?;
        }
        add(&mut root.counters.unmeasured, lower_bound)?;
        add(&mut global.unmeasured, lower_bound)?;
        root.counters.incomplete |= incomplete;
        global.incomplete |= incomplete;
        root.owner = owner.get();
        self.roots[index] = root;
        self.global = global;
        Ok(())
    }

    /// Unknown omitted extent stays null; no child count or byte estimate is invented.
    pub fn mark_incomplete(
        &mut self,
        owner: WorkspaceId,
        lower_bound: u64,
    ) -> Result<(), StopReason> {
        self.record_omission(owner, false, true, lower_bound)
    }

    pub fn observe(
        &mut self,
        owner: WorkspaceId,
        metadata: &ScanEntry,
    ) -> Result<ObservationOutcome, StopReason> {
        if let ReadDecision::Skip(reason) = ScanReadPolicy::evaluate(&metadata.facts) {
            let incomplete = !matches!(
                reason,
                super::ReadSkipReason::SymlinkExcluded
                    | super::ReadSkipReason::MountExcluded
                    | super::ReadSkipReason::SpecialExcluded
            );
            self.record_omission(owner, true, incomplete, u64::from(incomplete))?;
            return Ok(ObservationOutcome::Unmeasured);
        }
        let Some(identity) = metadata.identity else {
            self.mark_incomplete(owner, 1)?;
            return Ok(ObservationOutcome::Unmeasured);
        };
        if metadata.facts.kind == EntryKind::RegularFile && metadata.logical_bytes.is_none() {
            self.mark_incomplete(owner, 1)?;
            return Ok(ObservationOutcome::Unmeasured);
        }
        let slot_index = self.locate(identity)?;
        let index = self.root_index(owner)?;
        let mut root = self.roots[index];
        root.owner = owner.get();
        let mut global = self.global;
        let mut slot = self.slots[slot_index];
        let outcome = if slot.state != 0 {
            add(&mut root.counters.references, 1)?;
            add(&mut global.references, 1)?;
            let first_owner =
                WorkspaceId::try_from(slot.owner).expect("occupied slot has an owner");
            let first_index = self.root_index(first_owner)?;
            if owner != first_owner {
                root.covered_by |= 1 << first_index;
            }
            let directory_coverage = slot.directory_coverage();
            if directory_coverage.is_some_and(|coverage| coverage != Coverage::CompleteWithinPolicy)
            {
                root.counters.incomplete = true;
                global.incomplete = true;
            }
            if directory_coverage.is_some() {
                slot.dependents |= 1 << index;
            }
            if slot.state == 6 {
                root.counters.allocation_incomplete = true;
            }
            ObservationOutcome::Duplicate {
                owner: first_owner,
                directory_coverage,
            }
        } else {
            if self.records >= self.max_records {
                return Err(StopReason::IdentityLimit);
            }
            root.counters.observe(metadata)?;
            global.observe(metadata)?;
            slot = IdentitySlot {
                identity,
                owner: owner.get(),
                dependents: 1 << index,
                state: if metadata.facts.kind == EntryKind::Directory {
                    2
                } else if metadata.allocated_bytes.is_none() {
                    6
                } else {
                    1
                },
                padding: [0; 11],
            };
            if metadata.facts.kind == EntryKind::Directory {
                ObservationOutcome::Directory
            } else {
                ObservationOutcome::File
            }
        };
        // All fallible calculations precede identity, ownership and metric mutation.
        if self.slots[slot_index].state == 0 {
            self.records += 1;
        }
        self.slots[slot_index] = slot;
        self.roots[index] = root;
        self.global = global;
        Ok(outcome)
    }

    pub fn snapshot(&self) -> AggregateMetrics {
        self.global.snapshot()
    }

    pub fn workspace_snapshot(&self, owner: WorkspaceId) -> Option<WorkspaceAccounting> {
        let root = self.roots.iter().find(|root| root.owner == owner.get())?;
        let snapshot = root.counters.snapshot();
        let mut covered_by_workspace_ids: Vec<_> = self
            .roots
            .iter()
            .enumerate()
            .filter(|(index, _)| root.covered_by & (1 << index) != 0)
            .map(|(_, root)| {
                WorkspaceId::try_from(root.owner).expect("covered owner is registered")
            })
            .collect();
        covered_by_workspace_ids.sort_by_key(|owner| owner.get());
        Some(WorkspaceAccounting {
            metrics: snapshot.metrics,
            coverage: snapshot.coverage,
            covered_by_workspace_ids,
        })
    }

    /// Only explicit completion makes a directory complete; repeats before or
    /// after incomplete completion retain partial coverage without a child tree.
    pub fn finish_directory(&mut self, identity: ObjectIdentity, coverage: Coverage) {
        let Ok(index) = self.locate(identity) else {
            return;
        };
        let first_owner = self.slots[index].owner;
        if self.slots[index].state == 2 {
            self.global.pending_directories -= 1;
            self.roots
                .iter_mut()
                .find(|root| root.owner == first_owner)
                .expect("directory owner is registered")
                .counters
                .pending_directories -= 1;
        }
        let slot = &mut self.slots[index];
        if slot.directory_coverage().is_none() {
            return;
        }
        if matches!(slot.state, 4 | 5) {
            return;
        }
        slot.state = match coverage {
            Coverage::CompleteWithinPolicy => 3,
            Coverage::Partial => 4,
            Coverage::Unavailable => 5,
        };
        if coverage != Coverage::CompleteWithinPolicy {
            for (index, root) in self.roots.iter_mut().enumerate() {
                if slot.dependents & (1 << index) != 0 {
                    root.counters.incomplete = true;
                }
            }
            self.global.incomplete = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::{CloudFact, MountFact, ReadFacts, ScanMode};

    fn owner(id: i64) -> WorkspaceId {
        WorkspaceId::try_from(id).unwrap()
    }

    fn entry(id: u64, bytes: u64, allocated: Option<u64>) -> ScanEntry {
        ScanEntry {
            name: "synthetic".into(),
            identity: Some(ObjectIdentity {
                volume_uuid: [1; 16],
                file_id: id,
                birth_seconds: 0,
                birth_nanoseconds: 0,
            }),
            facts: ReadFacts {
                kind: EntryKind::RegularFile,
                in_read_scope: true,
                filesystem_supported: true,
                mount: MountFact::Same,
                cloud: CloudFact::Materialized,
                root_too_broad: false,
            },
            logical_bytes: Some(bytes),
            allocated_bytes: allocated,
        }
    }

    // Catches counting hard links twice, volume collisions, and re-enumerating directories.
    #[test]
    fn scan_accounting_deduplicates() {
        let mut accounting = ScanAccounting::new(ScanLimits::for_mode(ScanMode::Quick)).unwrap();
        let file = entry(1, 12, Some(512));
        assert_eq!(
            accounting.observe(owner(7), &file),
            Ok(ObservationOutcome::File)
        );
        assert_eq!(
            accounting.observe(owner(9), &file),
            Ok(ObservationOutcome::Duplicate {
                owner: owner(7),
                directory_coverage: None,
            })
        );
        assert_eq!(accounting.snapshot().metrics.observed_logical_bytes.0, 12);
        let later = accounting.workspace_snapshot(owner(9)).unwrap();
        assert_eq!(later.metrics.observed_logical_bytes.0, 0);
        assert_eq!(later.covered_by_workspace_ids, vec![owner(7)]);
        assert_eq!(
            accounting
                .workspace_snapshot(owner(7))
                .unwrap()
                .metrics
                .observed_logical_bytes
                .0,
            12
        );
        let mut other_volume = file.clone();
        other_volume.identity.as_mut().unwrap().volume_uuid = [2; 16];
        accounting.observe(owner(9), &other_volume).unwrap();
        assert_eq!(accounting.snapshot().metrics.observed_logical_bytes.0, 24);
        let mut directory = entry(2, 100, Some(512));
        directory.facts.kind = EntryKind::Directory;
        assert_eq!(
            accounting.observe(owner(7), &directory),
            Ok(ObservationOutcome::Directory)
        );
        accounting.finish_directory(directory.identity.unwrap(), Coverage::Partial);
        assert_eq!(
            accounting.observe(owner(9), &directory),
            Ok(ObservationOutcome::Duplicate {
                owner: owner(7),
                directory_coverage: Some(Coverage::Partial),
            })
        );
        assert_eq!(accounting.snapshot().metrics.unique_directories.0, 1);
        assert_eq!(accounting.snapshot().metrics.observed_logical_bytes.0, 24);
        assert_eq!(
            accounting.workspace_snapshot(owner(9)).unwrap().coverage,
            Coverage::Partial
        );
    }

    // Catches a full table forgetting earlier identities or committing a failed observation.
    #[test]
    fn scan_accounting_bounds_are_atomic() {
        let mut accounting = ScanAccounting::with_capacity(2, 2).unwrap();
        accounting
            .observe(owner(7), &entry(1, 12, Some(512)))
            .unwrap();
        accounting
            .observe(owner(7), &entry(2, 13, Some(512)))
            .unwrap();
        let before = accounting.snapshot();
        assert_eq!(
            accounting.observe(owner(9), &entry(3, 14, Some(512))),
            Err(StopReason::IdentityLimit)
        );
        assert_eq!(accounting.snapshot(), before);
        assert!(accounting.workspace_snapshot(owner(9)).is_none());
        assert!(matches!(
            accounting.observe(owner(9), &entry(1, 12, Some(512))),
            Ok(ObservationOutcome::Duplicate { .. })
        ));
        assert_eq!(accounting.snapshot().metrics.observed_logical_bytes.0, 25);

        let mut overflow = ScanAccounting::with_capacity(4, 4).unwrap();
        overflow
            .observe(owner(7), &entry(1, u64::MAX, Some(u64::MAX)))
            .unwrap();
        let before = overflow.snapshot();
        assert_eq!(
            overflow.observe(owner(9), &entry(2, 1, Some(0))),
            Err(StopReason::QuantityOverflow)
        );
        assert_eq!(overflow.snapshot(), before);
        assert!(overflow.workspace_snapshot(owner(9)).is_none());
        assert_eq!(
            overflow.observe(owner(9), &entry(2, 0, Some(0))),
            Ok(ObservationOutcome::File)
        );
        let before = overflow.snapshot();
        assert_eq!(
            overflow.observe(owner(9), &entry(3, 0, Some(1))),
            Err(StopReason::QuantityOverflow)
        );
        assert_eq!(overflow.snapshot(), before);
        assert_eq!(
            overflow.observe(owner(9), &entry(3, 0, Some(0))),
            Ok(ObservationOutcome::File)
        );
    }

    // Catches substituting logical bytes/zero for unknown allocation and mixing coverage.
    #[test]
    fn scan_accounting_bounds_preserve_unknown_allocation() {
        let mut accounting = ScanAccounting::with_capacity(4, 4).unwrap();
        accounting.observe(owner(7), &entry(1, 12, None)).unwrap();
        let unknown = accounting.snapshot();
        assert_eq!(unknown.metrics.observed_allocated_bytes, None);
        assert_eq!(unknown.metrics.allocated_coverage, Coverage::Unavailable);
        accounting
            .observe(owner(7), &entry(2, 13, Some(512)))
            .unwrap();
        let snapshot = accounting.snapshot();
        assert_eq!(snapshot.metrics.observed_logical_bytes.0, 25);
        assert_eq!(
            snapshot.metrics.observed_allocated_bytes,
            Some(DecimalQuantity(512))
        );
        assert_eq!(snapshot.metrics.allocation_measured_files.0, 1);
        assert_eq!(snapshot.metrics.allocation_unknown_files.0, 1);
        assert_eq!(
            snapshot.metrics.logical_coverage,
            Coverage::CompleteWithinPolicy
        );
        assert_eq!(snapshot.metrics.allocated_coverage, Coverage::Partial);
        assert_eq!(snapshot.coverage, Coverage::Partial);
    }

    // Catches production payload storage drifting beyond the bounded mode budgets.
    #[test]
    fn scan_accounting_bounds_use_fixed_slots_and_record_caps() {
        assert_eq!(std::mem::size_of::<IdentitySlot>(), 64);
        for (mode, slots, bytes, records) in [
            (ScanMode::Quick, 65_536, 4_194_304, 25_000),
            (ScanMode::Deep, 524_288, 33_554_432, 250_000),
        ] {
            let accounting = ScanAccounting::new(ScanLimits::for_mode(mode)).unwrap();
            assert_eq!(accounting.slots.len(), slots);
            assert_eq!(std::mem::size_of_val(accounting.slots.as_slice()), bytes);
            assert_eq!(accounting.max_records, records);
        }
        let mut accounting = ScanAccounting::with_capacity(4, 2).unwrap();
        accounting.observe(owner(7), &entry(1, 0, Some(0))).unwrap();
        accounting.observe(owner(7), &entry(2, 0, Some(0))).unwrap();
        assert_eq!(
            accounting.observe(owner(7), &entry(3, 0, Some(0))),
            Err(StopReason::IdentityLimit)
        );
    }

    // Catches publishing a pending directory as complete, or upgrading a repeat.
    #[test]
    fn scan_accounting_pending_directory_is_incomplete_until_finished() {
        let mut accounting = ScanAccounting::with_capacity(4, 4).unwrap();
        let mut directory = entry(1, 200, Some(512));
        directory.facts.kind = EntryKind::Directory;
        accounting.observe(owner(7), &directory).unwrap();
        assert_eq!(accounting.snapshot().coverage, Coverage::Partial);
        accounting.finish_directory(directory.identity.unwrap(), Coverage::CompleteWithinPolicy);
        assert_eq!(
            accounting.snapshot().coverage,
            Coverage::CompleteWithinPolicy
        );
        assert_eq!(
            accounting.snapshot().metrics.observed_allocated_bytes,
            Some(DecimalQuantity(0))
        );
        directory.identity.as_mut().unwrap().file_id = 2;
        accounting.observe(owner(7), &directory).unwrap();
        accounting.observe(owner(9), &directory).unwrap();
        accounting.finish_directory(directory.identity.unwrap(), Coverage::Partial);
        accounting.finish_directory(directory.identity.unwrap(), Coverage::CompleteWithinPolicy);
        assert_eq!(
            accounting.workspace_snapshot(owner(9)).unwrap().coverage,
            Coverage::Partial
        );
        assert_eq!(
            accounting.workspace_snapshot(owner(7)).unwrap().coverage,
            Coverage::Partial
        );
    }

    // Catches treating unknown scope as a complete policy exclusion or fabricated bytes.
    #[test]
    fn scan_accounting_unmeasured_scope_is_partial_but_fixed_exclusions_are_complete() {
        let mut accounting = ScanAccounting::with_capacity(4, 4).unwrap();
        let mut excluded = entry(1, 12, Some(512));
        excluded.facts.kind = EntryKind::Symlink;
        accounting.observe(owner(7), &excluded).unwrap();
        assert_eq!(
            accounting.snapshot().coverage,
            Coverage::CompleteWithinPolicy
        );
        excluded.facts.kind = EntryKind::Directory;
        excluded.facts.cloud = CloudFact::Dataless;
        accounting.observe(owner(7), &excluded).unwrap();
        let snapshot = accounting.snapshot();
        assert_eq!(snapshot.coverage, Coverage::Partial);
        assert_eq!(snapshot.metrics.unmeasured_bytes, None);
        assert_eq!(snapshot.metrics.policy_excluded_entries, DecimalQuantity(2));
        assert_eq!(
            snapshot.metrics.unmeasured_entries_lower_bound,
            DecimalQuantity(1)
        );
        assert_eq!(snapshot.metrics.unique_directories, DecimalQuantity(0));
        let mut missing = entry(2, 12, Some(512));
        missing.identity = None;
        assert_eq!(
            accounting.observe(owner(9), &missing),
            Ok(ObservationOutcome::Unmeasured)
        );
        assert_eq!(
            accounting.snapshot().metrics.unmeasured_entries_lower_bound,
            DecimalQuantity(2)
        );
    }
}
