import { useQuery } from "@tanstack/react-query";
import { Copy } from "lucide-react";
import { toast } from "sonner";

import { Button } from "../../components/ui/button";
import { Input } from "../../components/ui/input";
import { ROUTES } from "../../lib/routes";
import { formatCents } from "../pricing/pricing";
import { PanelCard } from "../shared/panel-card";
import { referralsQuery } from "./billing-api";

/** The host's invite link: a month of Pro each once the invited host pays. */
export function InviteCard({ paying }: { paying: boolean }) {
  const query = useQuery(referralsQuery);
  const referrals = query.data;
  if (!referrals?.code) return null;
  const link = `${window.location.origin}${ROUTES.register}?ref=${referrals.code}`;
  const credit = formatCents(referrals.credit_cents);
  const copy = () =>
    navigator.clipboard.writeText(link).then(
      () => toast.success("Invite link copied"),
      () => toast.error("The link could not be copied. Select it and copy it."),
    );

  return (
    <PanelCard
      title="Give a month, get a month"
      description={`When someone signs up with your link and starts paying, you each get ${credit} off, a month of Pro.${paying ? "" : " Yours arrives once you are on a paid plan."}`}
    >
      <div className="flex flex-wrap gap-2">
        <Input
          readOnly
          value={link}
          aria-label="Your invite link"
          className="min-w-0 flex-1 basis-64"
          onFocus={(event) => event.currentTarget.select()}
        />
        <Button variant="outline" onClick={copy}>
          <Copy />
          Copy link
        </Button>
      </div>
      {referrals.invited > 0 ? (
        <p className="text-sm text-muted-foreground">
          {referrals.invited} signed up with it, {referrals.paying} of them paying.
        </p>
      ) : null}
    </PanelCard>
  );
}
