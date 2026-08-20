# voice-local-audio

Binds an admitted media session to this machine's speaker and microphone.

`sip.dial` places a call and hands back a `TelephonySession`. Until something reads that session the
call is connected and inaudible — RTP flows to nobody. This is the something.

## It names no protocol

SIP and RTVBP are different protocols, and neither is a stage of the other. What they have in common
is that **both produce a generalized media session** — a `domain::voice::TelephonySession` — and that
is the only thing this crate knows about. It cannot tell which protocol established the call it is
carrying, and it has no code path per protocol.

A binding that named SIP would have to be written a second time the day the other one needs a
speaker.

## No transcoding happens here

You might expect G.711 at this seam, because that is what the trunk negotiates. It is not.
`MediaDescriptor::validate` admits exactly one profile — `pcm_s16le`, 8 kHz, mono, 20 ms, 320 bytes —
so the driver has already decoded to PCM by the time a frame crosses `TelephonySession`. Those bytes
are byte-identical to what the device wants.

The profile is **asserted at bind time**, not assumed: a future second profile arrives as a refusal
to bind rather than as audio played at the wrong rate.

## Shape

The session is async; the device is not, because every device is a pipe to a subprocess. So each
direction is an async task paired with a blocking one, joined by a bounded channel:

```text
far end ──▶ read_input()  ──▶ [queue] ──▶ playback.write() ──▶ speaker
far end ◀── write_output() ◀── [queue] ◀── capture.read()   ◀── microphone
```

**The queues drop rather than grow.** A speaker that falls behind must not accumulate a backlog: in a
live call, audio that arrives late is worthless, and buffering it converts a brief stall into
permanent drift where every later word is played at the wrong time. One second of frames absorbs a
scheduling hiccup; past that, dropping keeps the conversation aligned with the clock. A listener
notices growing delay long before they notice one lost packet.

## Teardown

`stop()` aborts the two async tasks **only**. That is deliberate and sufficient: aborting drops the
channel ends the async side owns, which closes the queues, which is what the two blocking threads
observe. The microphone thread is parked in a blocking read at the time, so it notices one frame
later — 20 ms — and then kills its recorder.

Trying to abort a blocking task instead would do nothing at all, and the recorder would outlive the
call.

## Two failure modes it is built against

1. **A partial read is not a frame.** A device read may return less than asked for. Sending a short
   buffer as a frame renders as a click, and leaves every later frame boundary misaligned.
   `fill` reads whole frames or reports the device ended; `a_partial_read_is_never_sent_as_a_frame`
   pins it.
2. **A non-monotonic outbound sequence drops the call.** The session refuses it, so the sequence
   starts at one and only ever rises.
   `the_microphone_reaches_the_far_end_with_a_rising_sequence` pins it.

## Tests

No test opens a device, a socket, or a call. The session is a fake implementing the same trait — the
binding is supposed to be unable to tell what established it, and that fake is what asserts it — and
the device is `driver_audio::NullAudioDevice`, so the whole path is exercised on a host with no sound
stack.
