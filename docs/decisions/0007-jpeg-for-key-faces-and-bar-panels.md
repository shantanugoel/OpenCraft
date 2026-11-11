# 0007: Send JPEG to key faces and bar panels

- Status: Accepted
- Date: 2026-11-11

## Context

Phase 1 forces this decision. The device accepts PNG and JPEG for a key face
and for a bar panel. The probe sent PNG until now. The facts are in
[kb/streamdock-protocol.md](../kb/streamdock-protocol.md#image-transfer) and
in [kb/n4-pro-hardware.md](../kb/n4-pro-hardware.md#surfaces).

The measurements gave no difference in speed that a user can see:

- The device shows PNG and JPEG after the same time.
- A full repaint of 14 images is written in 24 to 45 ms in each format.
- The encoder needs less than 0.2 ms for one image in each format.

Thus the decision depends on the quality of the image and on the faults that
each format can cause.

- The owner compared PNG and JPEG with quality 90 on the device, with small
  text of 9 to 13 pixels. The owner saw no difference.
- The device shows no slot image that is larger than approximately 58 000
  bytes, and it gives no error. A PNG of a bar panel can be larger than that:
  59 316 bytes for noise. A JPEG with quality 90 cannot: 37 460 bytes for
  noise.
- For real icons, a JPEG with quality 90 has approximately half of the bytes
  of the PNG. For key faces with text on a flat colour, the two formats have
  the same size.
- The vendor SDK sends JPEG for each key image that has no transparency.
  `mirajazz` and `opendeck-akp05` send JPEG. JPEG is the format that most
  software sends to this firmware.

## Decision

The device layer sends JPEG with quality 90 to each key face and each bar
panel. The descriptor has JPEG for these surfaces. The constant
`SLOT_JPEG_QUALITY` in `crates/core/src/image.rs` holds the quality.

The background stays JPEG with quality 60, the value of the vendor SDK. This
decision does not change it.

## Results

- No image that the user supplies can be too large for the device. A test
  makes sure that noise at this quality stays below the measured limit.
- One encoder serves all surfaces.
- The image on the device is not exactly the image in the GUI (rule 11): JPEG
  changes pixels a little. The owner saw no difference at quality 90.
- OpenCraft does not use the transparency of PNG. The device can draw a PNG
  with transparency on top of the background. The engine does not need this:
  it makes the full image of each surface. If a later feature needs
  transparency on the device, write a new record.
- The device layer keeps `encode_as`, which encodes in a format that the
  descriptor does not give. The probe uses it for measurements.
- Look at this decision again if a device model or a firmware version shows
  JPEG with visible faults, or if the limit for a slot image is different on a
  different model.
