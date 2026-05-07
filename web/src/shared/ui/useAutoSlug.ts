import { useState } from 'react';
import { slugify } from '../lib/text';

export function useAutoSlug(initialValue = '') {
  const [value, setValue] = useState(initialValue);
  const [slug, setSlugValue] = useState(slugify(initialValue));
  const [slugEdited, setSlugEdited] = useState(false);

  return {
    value,
    slug,
    setValue(nextValue: string) {
      setValue(nextValue);
      if (!slugEdited) {
        setSlugValue(slugify(nextValue));
      }
    },
    setSlug(nextSlug: string) {
      setSlugEdited(nextSlug.length > 0);
      setSlugValue(slugify(nextSlug));
    },
  };
}
