export type SearchParams = Promise<Record<string, string | string[] | undefined>>;

export function one(value: string | string[] | undefined): string {
  return (Array.isArray(value) ? value[0] : value)?.trim() ?? "";
}
