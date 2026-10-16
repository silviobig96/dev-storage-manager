import { expect, it } from 'vitest';
import { formatScanBytes, exactScanBytes } from './scanFormatting';

it.each([
  [null, 'Not available', 'Not available'],
  ['0', '0 B', '0 bytes'],
  ['1073741824', '1 GiB', '1,073,741,824 bytes'],
  ['9007199254740993', '8 PiB', '9,007,199,254,740,993 bytes'],
  ['1536', '1.5 KiB', '1,536 bytes'],
])(
  'formats %s without losing exact bytes or turning unavailable into zero',
  (value, formatted, exact) => {
    expect(formatScanBytes(value)).toBe(formatted);
    expect(exactScanBytes(value)).toBe(exact);
  },
);
