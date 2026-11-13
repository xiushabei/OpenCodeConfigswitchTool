export const OMO_SECTIONS = ["agents", "categories"] as const;

export type OmoSection = (typeof OMO_SECTIONS)[number];

export interface ModelOption {
  readonly id: string;
  readonly provider: string;
  readonly model: string;
}

export interface ModelCatalogue {
  readonly models: readonly ModelOption[];
  readonly directory: string;
}

export interface ModelAssignment {
  readonly section: OmoSection;
  readonly key: string;
  readonly model: string | null;
}

export function isOmoSection(value: string): value is OmoSection {
  return value === "agents" || value === "categories";
}
