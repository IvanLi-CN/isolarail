import { ThemeContext, useLang, useSite } from "@rspress/core/runtime";
import { IconMoon, IconSun, SvgWrapper } from "@rspress/core/theme";
import { useContext, useRef } from "react";
import { flushSync } from "react-dom";

import { toggleAppearanceTheme } from "./appearanceTransition";

export function SwitchAppearance({ onClick }: { onClick?: () => void }) {
  const { theme, setTheme = () => {} } = useContext(ThemeContext);
  const { site } = useSite();
  const lang = useLang();
  const buttonRef = useRef<HTMLButtonElement | null>(null);

  const handleClick = () => {
    const trigger = buttonRef.current;
    const nextTheme = theme === "dark" ? "light" : "dark";

    if (!trigger) {
      setTheme(nextTheme);
      onClick?.();
      return;
    }

    toggleAppearanceTheme({
      currentTheme: theme === "dark" ? "dark" : "light",
      animationEnabled: Boolean(site?.themeConfig?.enableAppearanceAnimation),
      triggerBounds: trigger.getBoundingClientRect(),
      setTheme,
      onToggle: onClick,
      flushSync,
      environment: {
        innerWidth: window.innerWidth,
        innerHeight: window.innerHeight,
        prefersReducedMotion: window.matchMedia(
          "(prefers-reduced-motion: reduce)",
        ).matches,
        startViewTransition:
          typeof document.startViewTransition === "function"
            ? document.startViewTransition.bind(document)
            : undefined,
        animateRoot: (keyframes, options) =>
          document.documentElement.animate(keyframes, options),
        appendStyle: (cssText) => {
          const styleDom = document.createElement("style");
          styleDom.textContent = cssText;
          document.head.appendChild(styleDom);
          return {
            remove: () => {
              styleDom.remove();
            },
          };
        },
      },
    });
  };

  const ariaLabel = lang === "zh" ? "切换主题" : "Toggle theme";

  return (
    <button
      ref={buttonRef}
      aria-label={ariaLabel}
      className="rp-switch-appearance docs-switch-appearance"
      onClick={handleClick}
      type="button"
    >
      <SvgWrapper
        className="rp-switch-appearance__icon rp-switch-appearance__icon--sun"
        fill="currentColor"
        icon={IconSun}
      />
      <SvgWrapper
        className="rp-switch-appearance__icon rp-switch-appearance__icon--moon"
        fill="currentColor"
        icon={IconMoon}
      />
    </button>
  );
}
