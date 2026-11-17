// One small store for module state the UI reads: a value, a way to change it, and a hook that renders
// again when it changes. `use(select)` reads only part of it, so a component renders again only when
// that part changes; a selector returns a plain value or one already in the store, never a new object.
import { useSyncExternalStore } from "react";

export type Store<T> = {
  get: () => T;
  /** Replaces the value, or works it out from the one before. Every reader hears of it. */
  set: (next: T | ((prev: T) => T)) => void;
  subscribe: (f: () => void) => () => void;
  use: <S = T>(select?: (value: T) => S) => S;
};

export function createStore<T>(initial: T): Store<T> {
  let value = initial;
  const listeners = new Set<() => void>();
  const get = () => value;
  const subscribe = (f: () => void) => {
    listeners.add(f);
    return () => { listeners.delete(f); };
  };
  const set = (next: T | ((prev: T) => T)) => {
    value = typeof next === "function" ? (next as (prev: T) => T)(value) : next;
    listeners.forEach((f) => f());
  };
  function use<S = T>(select?: (value: T) => S): S {
    return useSyncExternalStore(subscribe, select ? () => select(value) : (get as unknown as () => S));
  }
  return { get, set, subscribe, use };
}
