import type { Snippet } from "svelte";

/** Colonna di DataTable: `sort` dà il valore di ordinamento, `cell` disegna la cella. */
export type Col<T> = {
  key: string;
  label: string;
  width: string;
  align?: "right";
  sort?: (r: T) => number | string;
  cell: Snippet<[T]>;
};
