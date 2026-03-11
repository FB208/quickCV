import type { Folder, TemplateItem } from "./types";

export const SORT_GAP = 1024;

export type DropPosition = "before" | "after";

export const compareFolders = (left: Folder, right: Folder): number => {
  return (
    left.sortOrder - right.sortOrder ||
    left.sortUpdatedAt - right.sortUpdatedAt ||
    left.updatedAt - right.updatedAt ||
    left.id.localeCompare(right.id)
  );
};

export const compareTemplates = (left: TemplateItem, right: TemplateItem): number => {
  return (
    left.sortOrder - right.sortOrder ||
    left.sortUpdatedAt - right.sortUpdatedAt ||
    left.updatedAt - right.updatedAt ||
    left.id.localeCompare(right.id)
  );
};

export const sortFolders = (folders: Folder[]): Folder[] => {
  return [...folders].sort(compareFolders);
};

export const sortTemplates = (templates: TemplateItem[]): TemplateItem[] => {
  return [...templates].sort(compareTemplates);
};

export const nextSortOrder = (items: Array<Folder | TemplateItem>): number => {
  const maxValue = items.reduce((maxValue, item) => {
    return Math.max(maxValue, item.sortOrder || 0);
  }, 0);
  return maxValue + SORT_GAP;
};

export const moveIds = (
  ids: string[],
  sourceId: string,
  targetId: string,
  position: DropPosition,
): string[] => {
  if (sourceId === targetId) {
    return ids;
  }

  const sourceIndex = ids.indexOf(sourceId);
  const targetIndex = ids.indexOf(targetId);
  if (sourceIndex < 0 || targetIndex < 0) {
    return ids;
  }

  const nextIds = [...ids];
  const [movedId] = nextIds.splice(sourceIndex, 1);
  const normalizedTargetIndex = nextIds.indexOf(targetId);
  const insertIndex = position === "before" ? normalizedTargetIndex : normalizedTargetIndex + 1;
  nextIds.splice(insertIndex, 0, movedId);
  return nextIds;
};
