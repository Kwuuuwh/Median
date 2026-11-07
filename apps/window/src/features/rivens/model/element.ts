/** Damage type added by each riven stat tag that adds one. */
const ELEMENTS: Record<string, string> = {
  WeaponImpactDamageMod: "impact",
  WeaponArmorPiercingDamageMod: "puncture",
  WeaponSlashDamageMod: "slash",
  WeaponFireDamageMod: "heat",
  WeaponFreezeDamageMod: "cold",
  WeaponElectricityDamageMod: "electricity",
  WeaponToxinDamageMod: "toxin",
  WeaponBlastDamageMod: "blast",
  WeaponCorrosiveDamageMod: "corrosive",
  WeaponGasDamageMod: "gas",
  WeaponMagneticDamageMod: "magnetic",
  WeaponRadiationDamageMod: "radiation",
  WeaponViralDamageMod: "viral",
};

/** Icon URL of the damage type a stat adds; null for a stat that adds none. */
export function elementPicture(tag: string): string | null {
  const element = ELEMENTS[tag];
  return element ? `/statuses/${element}.webp` : null;
}
