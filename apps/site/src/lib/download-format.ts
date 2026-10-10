/** Decimal megabytes match file sizes used for download totals (1 MB = 1,000,000 bytes). */
export const downloadSize = (bytes:number) => `${(bytes / 1_000_000).toFixed(bytes > 0 && bytes < 100_000 ? 3 : 1)} MB`;
export const downloadProgress = (loaded:number, total:number) => total
  ? `${Math.min(99, Math.floor(loaded / total * 100))}% · ${downloadSize(loaded)} of ${downloadSize(total)}`
  : "Preparing download…";
