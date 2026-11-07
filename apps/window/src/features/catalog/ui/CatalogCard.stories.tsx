import type { Meta, StoryObj } from "@storybook/react-vite";
import { CARD_WIDTH } from "@/features/items/ui/ItemCard";
import { sampleCollectible } from "@/mock/samples";
import { CatalogCard } from "./CatalogCard";

const meta = {
  title: "Catalog/Card",
  component: CatalogCard,
  parameters: { layout: "centered" },
  decorators: [
    (Story) => (
      <div style={{ width: CARD_WIDTH }}>
        <Story />
      </div>
    ),
  ],
} satisfies Meta<typeof CatalogCard>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Held: Story = {
  args: { row: sampleCollectible("/Lotus/Powersuits/Alchemist/LavosPrime", { owned: 2 }) },
};

export const Ready: Story = {
  args: {
    row: sampleCollectible("/Lotus/Weapons/Tenno/Bows/PrimeCernos/PrimeCernos", {
      owned: 0,
      ready: true,
    }),
  },
};

export const RankLocked: Story = {
  args: {
    row: sampleCollectible("/Lotus/Weapons/ClanTech/Energy/DeraVandal", {
      owned: 0,
      required: 12,
      reachable: false,
    }),
  },
};

export const Mastered: Story = {
  args: {
    row: sampleCollectible("/Lotus/Powersuits/AntiMatter/NovaPrime", { mastered: true }),
  },
};

export const Vaulted: Story = {
  args: {
    row: sampleCollectible("/Lotus/Weapons/Tenno/LongGuns/PrimeAcceltra/PrimeAcceltraWeapon", {
      owned: 0,
      vaulted: true,
    }),
  },
};

export const NoPicture: Story = {
  args: {
    row: sampleCollectible("/Lotus/Powersuits/Alchemist/LavosPrime", { image: null }),
  },
};
