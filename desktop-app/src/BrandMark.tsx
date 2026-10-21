import mark from "../resources/fermat.svg";

/** A transparent silhouette whose color follows the selected application theme. */
export function BrandMark({ className = "" }: { className?: string }) {
  return (
    <span
      aria-hidden="true"
      className={`brand-mark ${className}`}
      style={{ maskImage: `url("${mark}")`, WebkitMaskImage: `url("${mark}")` }}
    />
  );
}
