import { useMutation, useQuery } from "@tanstack/react-query";
import { CalendarDays, CircleAlert, Loader2 } from "lucide-react";
import { useState, type FormEvent } from "react";

import type { CreateBookingRequest } from "../../bindings/CreateBookingRequest";
import type { TimeSlot } from "../../bindings/TimeSlot";
import { Alert, AlertDescription } from "../../components/ui/alert";
import { Button } from "../../components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "../../components/ui/card";
import { Input } from "../../components/ui/input";
import { Label } from "../../components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "../../components/ui/select";
import { api } from "../../lib/api";
import { QUERY_KEYS } from "../../lib/query-keys";

type Props = { hostUnid: string };

export function BookingWidget({ hostUnid }: Props) {
  const [slot, setSlot] = useState("");
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const slotsQuery = useQuery({
    queryKey: QUERY_KEYS.publicSlots(hostUnid),
    queryFn: () => api<TimeSlot[]>(`/api/hosts/${hostUnid}/slots?days=14`),
    enabled: Boolean(hostUnid),
  });
  const bookingMutation = useMutation({
    mutationFn: (request: CreateBookingRequest) =>
      api(`/api/hosts/${hostUnid}/bookings`, {
        method: "POST",
        body: JSON.stringify(request),
      }),
    onSuccess: () => setSlot(""),
  });
  const slots = slotsQuery.data ?? [];
  const mutationError =
    bookingMutation.error instanceof Error ? bookingMutation.error.message : null;
  const queryError = slotsQuery.error instanceof Error ? slotsQuery.error.message : null;
  const message = bookingMutation.isSuccess
    ? "Call booked. Check your email."
    : (mutationError ?? queryError);

  if (!hostUnid) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Booking widget</CardTitle>
          <CardDescription>Public booking is not configured.</CardDescription>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-muted-foreground">
            Set VITE_PUBLIC_HOST_UNID to activate public booking.
          </p>
        </CardContent>
      </Card>
    );
  }

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    bookingMutation.reset();
    bookingMutation.mutate({
      lead_unid: null,
      calling_visit_unid: null,
      slot_start: slot,
      timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
      invitee_name: name,
      invitee_email: email,
    });
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <CalendarDays className="size-5 text-muted-foreground" />
          Book a call
        </CardTitle>
        <CardDescription>Choose a time that works for you.</CardDescription>
      </CardHeader>
      <CardContent>
        <form onSubmit={submit} className="grid gap-4">
          <div className="grid gap-2">
            <Label htmlFor="booking-slot">Available slot</Label>
            <Select value={slot} onValueChange={setSlot} required>
              <SelectTrigger id="booking-slot" className="w-full">
                <SelectValue placeholder="Choose a time" />
              </SelectTrigger>
              <SelectContent>
                {slots
                  .filter((item) => item.available)
                  .map((item) => (
                    <SelectItem key={item.start} value={item.start}>
                      {new Date(item.start).toLocaleString()}
                    </SelectItem>
                  ))}
              </SelectContent>
            </Select>
          </div>
          <div className="grid gap-2">
            <Label htmlFor="booking-name">Name</Label>
            <Input
              id="booking-name"
              value={name}
              onChange={(event) => setName(event.target.value)}
              required
            />
          </div>
          <div className="grid gap-2">
            <Label htmlFor="booking-email">Email</Label>
            <Input
              id="booking-email"
              type="email"
              value={email}
              onChange={(event) => setEmail(event.target.value)}
              required
            />
          </div>
          <Button disabled={!slots.length || bookingMutation.isPending}>
            {bookingMutation.isPending ? (
              <>
                <Loader2 className="size-4 animate-spin" />
                Booking...
              </>
            ) : (
              "Book call"
            )}
          </Button>
          {message ? (
            <Alert variant={mutationError || queryError ? "destructive" : "default"}>
              {mutationError || queryError ? <CircleAlert className="size-4" /> : null}
              <AlertDescription>{message}</AlertDescription>
            </Alert>
          ) : null}
        </form>
      </CardContent>
    </Card>
  );
}
