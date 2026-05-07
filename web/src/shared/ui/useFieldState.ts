import { useState } from 'react';

export function useFieldState<T>(initialValue: T) {
  const [value, setValue] = useState<T>(initialValue);
  return { value, setValue };
}
