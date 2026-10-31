import { useCallback, useEffect, useRef, useState, type DependencyList } from 'react';
import { normalizeError, type UiFailure } from './api';

export function useResource<T>(load: () => Promise<T>, dependencies: DependencyList) {
  const [data, setData] = useState<T | null>(null);
  const [error, setError] = useState<UiFailure | null>(null);
  const [busy, setBusy] = useState(true);
  const [revision, setRevision] = useState(0);
  const generation = useRef(0);
  useEffect(() => {
    const current = ++generation.current;
    setBusy(true);
    setError(null);
    const timer = setTimeout(
      () =>
        void load()
          .then((value) => {
            if (generation.current === current) setData(value);
          })
          .catch((error) => {
            if (generation.current === current) {
              setData(null);
              setError(normalizeError(error));
            }
          })
          .finally(() => {
            if (generation.current === current) setBusy(false);
          }),
      120,
    );
    return () => {
      clearTimeout(timer);
      generation.current++;
    };
  }, [...dependencies, revision]);
  const reload = useCallback(() => setRevision((v) => v + 1), []);
  return { data, busy, error, reload };
}
