import { type RefObject, useEffect, useState } from "react";

/** Whether the element is in the viewport. A loop off screen has nobody to watch it. */
export function useOnScreen(ref: RefObject<Element | null>): boolean {
  const [visible, setVisible] = useState(true);
  useEffect(() => {
    const element = ref.current;
    if (!element) return;
    const observer = new IntersectionObserver(([entry]) =>
      setVisible(entry?.isIntersecting ?? true),
    );
    observer.observe(element);
    return () => observer.disconnect();
  }, [ref]);
  return visible;
}
