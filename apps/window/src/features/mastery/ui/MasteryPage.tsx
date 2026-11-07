import { useState } from "react";
import { useMastery, usePlanToggle, useRoadmap } from "@/shared/data/mastery";
import { useT } from "@/shared/i18n";
import { Tab, TabList, Tabs } from "@/shared/ui/tabs";
import { ItemsView } from "./ItemsView";
import { PlanView } from "./PlanView";
import { RankStrip } from "./RankStrip";

type MasteryTab = "items" | "plan";

/** Mastery page: rank and progress to the next, tabs for all mastery items and for the plan. */
export function MasteryPage() {
  const t = useT();
  const standing = useMastery().data;
  const roadmap = useRoadmap().data;
  const toggle = usePlanToggle();
  const [tab, setTab] = useState<MasteryTab>("items");
  const [preview, setPreview] = useState(0);
  if (!standing) return null;

  const planned = new Set(roadmap?.planned.map((each) => each.item));
  const left = standing.masterable.filter((item) => item.stage !== "mastered").length;
  return (
    <Tabs
      value={tab}
      onValueChange={(value) => setTab(value as MasteryTab)}
      className="relative flex min-w-0 flex-1 flex-col"
    >
      <RankStrip standing={standing} preview={tab === "plan" ? (roadmap?.gain ?? 0) : preview} />
      <div className="flex shrink-0 items-end gap-4 border-line border-b px-6 pt-3">
        <TabList className="shrink-0 border-b-0">
          <Tab value="items" count={left}>
            {t("mastery-view-items")}
          </Tab>
          <Tab value="plan" count={planned.size}>
            {t("mastery-tab-plan")}
          </Tab>
        </TabList>
      </div>
      {tab === "items" && (
        <ItemsView
          standing={standing}
          planned={planned}
          onPlan={(item, on) => toggle.mutate({ item, on })}
          onPreview={setPreview}
        />
      )}
      {tab === "plan" && roadmap && <PlanView standing={standing} roadmap={roadmap} />}
    </Tabs>
  );
}
