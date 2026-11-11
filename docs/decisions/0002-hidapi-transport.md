# 0002: Use `hidapi` directly for the transport

- Status: Accepted
- Date: 2026-11-11. The code obeyed this decision from the first commit.

## Context

The first plan gave two options for the transport: the `mirajazz` crate, or
our own thin layer. The bring-up code used `hidapi` directly with our own
protocol code. That code passed the hardware gate.

## Decision

The device layer uses `hidapi` directly. OpenCraft owns the protocol code.

## Results

- The protocol code is small, and it operates on real hardware.
- The device layer has full control of which interface it opens. Rule 6 of the
  architecture needs this control.
- The call model is blocking. This agrees with
  [decision 0003](0003-blocking-device-layer.md).
- OpenCraft must add the special cases for other device models itself.
- `mirajazz` stays a reference for the protocol. Its licence is MPL-2.0. Do not
  copy its source text into OpenCraft files, which are MIT.
- Look at this decision again if the special cases for other models become a
  large part of the device layer.
