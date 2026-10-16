import { useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

import type { AvailabilityRule } from "../../bindings/AvailabilityRule";
import type { AvailabilityRuleInput } from "../../bindings/AvailabilityRuleInput";
import { errorMessage } from "../../lib/error-message";
import { clockTime, weekdayName } from "../../lib/format";
import { QUERY_KEYS } from "../../lib/query-keys";
import {
  createAvailabilityRule,
  removeAvailabilityRule,
  updateAvailabilityRule,
} from "../dashboard/host-api";

/** One range saved on a day, and the ranges it was folded together with. */
type SpanSave = {
  weekday: number;
  /** The range that takes the new times; none adds a range. */
  keep: AvailabilityRule | undefined;
  from: number;
  to: number;
  slotMinutes: number;
  /** Ranges now inside `keep`, removed. */
  drop: AvailabilityRule[];
};

const KEY = QUERY_KEYS.availability;

const input = (weekday: number, from: number, to: number, slotMinutes: number) => ({
  weekday,
  start_time: `${clockTime(from)}:00`,
  end_time: `${clockTime(to)}:00`,
  slot_minutes: slotMinutes,
});

const copyOf = (
  rule: AvailabilityRule,
  weekday = rule.weekday,
): AvailabilityRuleInput => ({
  weekday,
  start_time: rule.start_time,
  end_time: rule.end_time,
  slot_minutes: rule.slot_minutes,
});

/** Ids for ranges drawn before the server has given them one. */
let nextDraftId = -1;
const draft = (rule: AvailabilityRuleInput): AvailabilityRule => ({
  ...rule,
  id: nextDraftId--,
  host_unid: "",
});

/**
 * Changes to the weekly hours. Each one shows on the page at once and is put
 * back if the server refuses it.
 */
export function useHours() {
  const queryClient = useQueryClient();
  const current = () => queryClient.getQueryData<AvailabilityRule[]>(KEY) ?? [];
  /** Swaps in the hours a change will leave, and remembers the ones before. */
  const preview = async (next: (rules: AvailabilityRule[]) => AvailabilityRule[]) => {
    await queryClient.cancelQueries({ queryKey: KEY });
    const before = current();
    queryClient.setQueryData<AvailabilityRule[]>(KEY, next(before));
    return { before };
  };
  const putBack = (context: { before: AvailabilityRule[] } | undefined) => {
    if (context) queryClient.setQueryData(KEY, context.before);
  };
  const refresh = () => queryClient.invalidateQueries({ queryKey: KEY });

  const saveSpan = useMutation({
    mutationFn: async (save: SpanSave) => {
      const hours = input(save.weekday, save.from, save.to, save.slotMinutes);
      await (save.keep
        ? updateAvailabilityRule(save.keep.id, hours)
        : createAvailabilityRule(hours));
      await Promise.all(save.drop.map(removeAvailabilityRule));
    },
    onMutate: (save) =>
      preview((rules) => {
        const hours = input(save.weekday, save.from, save.to, save.slotMinutes);
        const dropped = new Set(save.drop.map((rule) => rule.id));
        const kept = rules.filter((rule) => !dropped.has(rule.id));
        return save.keep
          ? kept.map((rule) => (rule.id === save.keep?.id ? { ...rule, ...hours } : rule))
          : [...kept, draft(hours)];
      }),
    onError: (error, save, context) => {
      putBack(context);
      toast.error(
        `${weekdayName(save.weekday)}'s hours were not saved. ${errorMessage(error)}`,
      );
    },
    onSettled: refresh,
  });

  const restore = useMutation({
    mutationFn: (rules: AvailabilityRuleInput[]) =>
      Promise.all(rules.map(createAvailabilityRule)),
    onSuccess: () => toast.success("Hours put back"),
    onError: (error) => toast.error(`Hours were not put back. ${errorMessage(error)}`),
    onSettled: refresh,
  });

  const remove = useMutation({
    mutationFn: (rules: AvailabilityRule[]) =>
      Promise.all(rules.map(removeAvailabilityRule)),
    onMutate: (gone) =>
      preview((rules) => rules.filter((rule) => !gone.some((one) => one.id === rule.id))),
    onSuccess: (_, gone) =>
      toast.success("Hours removed", {
        action: {
          label: "Undo",
          onClick: () => restore.mutate(gone.map((rule) => copyOf(rule))),
        },
      }),
    onError: (error, _, context) => {
      putBack(context);
      toast.error(`Hours were not removed. ${errorMessage(error)}`);
    },
    onSettled: refresh,
  });

  /** Gives every day in `days` the hours of `source`, in place of their own. */
  const copyDay = useMutation({
    mutationFn: async ({ source, days }: { source: number; days: number[] }) => {
      const rules = current();
      const copies = rules.filter((rule) => rule.weekday === source);
      await Promise.all(
        rules.filter((rule) => days.includes(rule.weekday)).map(removeAvailabilityRule),
      );
      await Promise.all(
        days.flatMap((day) =>
          copies.map((rule) => createAvailabilityRule(copyOf(rule, day))),
        ),
      );
    },
    onSuccess: (_, { source, days }) =>
      toast.success(
        `${weekdayName(source)}'s hours copied to ${days.length} ${days.length === 1 ? "day" : "days"}`,
      ),
    onError: (error) =>
      toast.error(`Some days did not get the copied hours. ${errorMessage(error)}`),
    onSettled: refresh,
  });

  return { saveSpan, remove, copyDay };
}
