import heroDark from "../docs/public/isolarail-docs-hero-dark.jpg";
import heroLight from "../docs/public/isolarail-docs-hero.jpg";

type DocsThemeFigureProps = {
  className: string;
  darkAlt: string;
  lightAlt: string;
};

export function DocsThemeFigure({
  className,
  darkAlt,
  lightAlt,
}: DocsThemeFigureProps) {
  return (
    <figure className={className}>
      <img
        alt={lightAlt}
        className="docs-theme-figure-image docs-theme-figure-image--light"
        src={heroLight}
      />
      <img
        alt={darkAlt}
        className="docs-theme-figure-image docs-theme-figure-image--dark"
        src={heroDark}
      />
    </figure>
  );
}
