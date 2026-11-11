# 0001: Make a native application, not an OpenDeck plug-in

- Status: Accepted
- Date: 2026-11-11

## Context

An OpenDeck plug-in, `opendeck-akp05`, lists the VSD N4 Pro as a supported
device. Thus OpenCraft is not the only open-source software for this device.
See [../kb/prior-art.md](../kb/prior-art.md).

The owner examined this option and found three problems:

- OpenDeck is not made for this device family. Support comes from a community
  plug-in.
- The plug-in author does not test on macOS. The README says "Zero effort" for
  macOS.
- OpenDeck uses Tauri, which has a web view. The memory use is larger than
  that of a native application.

## Decision

OpenCraft is an independent native application. It is not an OpenDeck plug-in.

The reason for OpenCraft is a small footprint and full support for this device
family. The reason is not that no other software exists.

## Results

- OpenCraft has full control of the footprint and of the reliability.
- OpenCraft cannot use the OpenDeck plug-ins. A plug-in host is a possible item
  for later. See [../ROADMAP.md](../ROADMAP.md#not-in-a-phase).
- The public description of OpenCraft must give the footprint and the device
  support as the primary advantages.
- Look at this decision again if the product targets in the roadmap cannot be
  met.
