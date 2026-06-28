import logoIcon from "../assets/logo-icon.png";

export function BrandHeader() {
  return (
    <div className="brand-header" aria-label="Ponte Mesh">
      <img className="brand-header__icon" src={logoIcon} alt="" aria-hidden="true" />
      <span className="brand-header__wordmark">
        <span>Ponte</span>
        <span>Mesh</span>
      </span>
    </div>
  );
}
