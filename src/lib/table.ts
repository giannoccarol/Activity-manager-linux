import type { Snippet } from "svelte";

/** DataTable column: `sort` returns the sort value, `cell` renders the cell. */
export type Col<T> = {
  key: string;
  label: string;
  width: string;
  align?: "right";
  sort?: (r: T) => number | string;
  cell: Snippet<[T]>;
};
