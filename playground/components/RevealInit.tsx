'use client';
import { useEffect } from 'react';

export default function RevealInit() {
  useEffect(() => {
    const io = new IntersectionObserver(
      (entries) => {
        entries.forEach((e) => {
          if (e.isIntersecting) {
            e.target.classList.add('in');
            io.unobserve(e.target);
          }
        });
      },
      { threshold: 0.12, rootMargin: '0px 0px -40px 0px' }
    );
    document.querySelectorAll<HTMLElement>('.reveal').forEach((el, i) => {
      el.style.transitionDelay = `${Math.min(i % 4, 3) * 60}ms`;
      io.observe(el);
    });
    return () => io.disconnect();
  }, []);
  return null;
}
