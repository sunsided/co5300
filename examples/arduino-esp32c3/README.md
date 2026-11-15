# CO5300 demo for the Wokwi web editor

wokwi.com no longer compiles Rust online, so this demo uses an Arduino sketch. It speaks
raw 4-wire SPI with a D/C pin to the same CO5300 chip and draws the same test pattern as
the [Rust example](../esp32c3): four color bars and a red-to-blue gradient. The chip is
loaded from this repository's GitHub Release via the `dependencies` entry in
`diagram.json`.

Live project: <TODO>

To recreate it on wokwi.com:

1. Start a new **ESP32-C3** Arduino project.
2. Replace `sketch.ino` and `diagram.json` with the files from this directory.
3. Start the simulation. The CO5300 chip has no controls; its 410 × 502 display shows the
   test pattern.
