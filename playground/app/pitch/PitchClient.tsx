"use client";
import { useEffect } from "react";
import Lenis from "lenis";

export default function PitchClient() {
  useEffect(() => {
    const lenis = new Lenis({
      duration: 1.15,
      easing: (t: number) => Math.min(1, 1.001 - Math.pow(2, -10 * t)),
      smoothWheel: true,
      wheelMultiplier: 1,
      touchMultiplier: 1.6,
    });

    let rafId = 0;
    const loop = (time: number) => {
      lenis.raf(time);
      rafId = requestAnimationFrame(loop);
    };
    rafId = requestAnimationFrame(loop);

    const slides = Array.from(document.querySelectorAll<HTMLElement>(".slide"));
    const bar = document.getElementById("p-bar");
    const nav = document.getElementById("p-nav");

    // build side-nav dots
    const dots: HTMLAnchorElement[] = [];
    if (nav) {
      slides.forEach((s, i) => {
        const a = document.createElement("a");
        a.setAttribute("aria-label", `slide ${i + 1}`);
        a.addEventListener("click", (e) => {
          e.preventDefault();
          lenis.scrollTo(s, { offset: 0 });
        });
        nav.appendChild(a);
        dots.push(a);
      });
    }

    // reveal on scroll
    const io = new IntersectionObserver(
      (entries) => entries.forEach((e) => e.isIntersecting && e.target.classList.add("in")),
      { threshold: 0.16 },
    );
    document.querySelectorAll(".reveal").forEach((el) => io.observe(el));

    const currentIndex = () => {
      let cur = 0;
      const y = lenis.scroll;
      slides.forEach((s, i) => {
        if (s.offsetTop - window.innerHeight * 0.45 <= y) cur = i;
      });
      return cur;
    };

    lenis.on("scroll", ({ scroll, limit }: { scroll: number; limit: number }) => {
      if (bar) bar.style.width = (limit > 0 ? (scroll / limit) * 100 : 0) + "%";
      const cur = currentIndex();
      dots.forEach((d, i) => d.classList.toggle("on", i === cur));
    });

    const onKey = (e: KeyboardEvent) => {
      const cur = currentIndex();
      if (["ArrowRight", "ArrowDown", "PageDown", " "].includes(e.key)) {
        e.preventDefault();
        lenis.scrollTo(slides[Math.min(slides.length - 1, cur + 1)]);
      } else if (["ArrowLeft", "ArrowUp", "PageUp"].includes(e.key)) {
        e.preventDefault();
        lenis.scrollTo(slides[Math.max(0, cur - 1)]);
      } else if (e.key === "Home") {
        lenis.scrollTo(slides[0]);
      } else if (e.key === "End") {
        lenis.scrollTo(slides[slides.length - 1]);
      }
    };
    window.addEventListener("keydown", onKey);

    return () => {
      cancelAnimationFrame(rafId);
      io.disconnect();
      window.removeEventListener("keydown", onKey);
      lenis.destroy();
      if (nav) nav.innerHTML = "";
    };
  }, []);

  return null;
}
