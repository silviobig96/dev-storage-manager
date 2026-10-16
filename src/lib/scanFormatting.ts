/** Display precision never passes through floating point. Exact bytes accompany it. */
export function formatScanBytes(value: string | null): string {
  if (value === null) return 'Not available';
  const bytes = BigInt(value);
  const units = ['B', 'KiB', 'MiB', 'GiB', 'TiB', 'PiB', 'EiB'];
  let unit = 0;
  let divisor = 1n;
  while (bytes >= divisor * 1024n && unit < units.length - 1) {
    divisor *= 1024n;
    unit += 1;
  }
  const tenths = (bytes * 10n) / divisor;
  const fraction = tenths % 10n;
  return `${tenths / 10n}${fraction === 0n ? '' : `.${fraction}`} ${units[unit]}`;
}
export function exactScanBytes(value: string | null): string {
  return value === null
    ? 'Not available'
    : `${BigInt(value).toLocaleString('en-US')} bytes`;
}
