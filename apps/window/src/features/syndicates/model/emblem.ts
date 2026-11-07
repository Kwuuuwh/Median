/** Emblem file name of each syndicate, by its in-game tag. */
const EMBLEMS: Partial<Record<string, string>> = {
  SteelMeridianSyndicate: "steel_meridian",
  ArbitersSyndicate: "arbiters_hexis",
  CephalonSudaSyndicate: "cephalon_suda",
  PerrinSyndicate: "perrin_sequence",
  RedVeilSyndicate: "red_veil",
  NewLokaSyndicate: "new_loka",
  LibrarySyndicate: "cephalon_simaris",
  CetusSyndicate: "ostron",
  QuillsSyndicate: "quils",
  SolarisSyndicate: "solaris_united",
  VentKidsSyndicate: "ventkids",
  VoxSyndicate: "vox_solaris",
  EntratiSyndicate: "entrati",
  NecraloidSyndicate: "necraloid",
  ZarimanSyndicate: "holdfasts",
  EntratiLabSyndicate: "cavia",
  KahlSyndicate: "kahl_garrison",
  HexSyndicate: "hex",
};

/** URL of the syndicate's emblem; null when the window has no emblem for it. */
export function emblemOf(tag: string): string | null {
  const file = EMBLEMS[tag];
  return file ? `/syndicates/${file}.webp` : null;
}
