// CO5300 AMOLED demo for the Wokwi web editor (Arduino, ESP32-C3).
//
// Talks to the CO5300 Wokwi chip over raw 4-wire SPI with a D/C pin and draws the same test
// pattern as the Rust example in ../esp32c3, printing the same lines.
// Wiring: SCL = GPIO4, SDI = GPIO5, CSX = GPIO6, DCX = GPIO7, RESX = GPIO10.
// The panel is 410 × 502, as on the Kode Dot.

#include <SPI.h>

constexpr uint16_t WIDTH = 410;
constexpr uint16_t HEIGHT = 502;

constexpr uint8_t CMD_SLPOUT = 0x11;
constexpr uint8_t CMD_DISPON = 0x29;
constexpr uint8_t CMD_CASET = 0x2A;
constexpr uint8_t CMD_RASET = 0x2B;
constexpr uint8_t CMD_RAMWR = 0x2C;
constexpr uint8_t CMD_COLMOD = 0x3A;
constexpr uint8_t CMD_WRDISBV = 0x51;
constexpr uint8_t CMD_SETSPIMODE = 0xC4;
constexpr uint8_t CMD_PAGESEL = 0xFE;

constexpr uint8_t PAGE_USER = 0x00;
// SETSPIMODE: SPI_WRAM, frame memory writes over SPI.
constexpr uint8_t SPI_MODE_WRAM = 0x80;
// COLMOD: RGB565 in both VIPF and IFPF.
constexpr uint8_t COLMOD_RGB565 = 0x55;
constexpr uint8_t BRIGHTNESS_MAX = 0xFF;

constexpr uint32_t RESET_LOW_MS = 10;
constexpr uint32_t RESET_WAIT_MS = 120;
constexpr uint32_t SLEEP_OUT_TO_DISPLAY_ON_MS = 60;

constexpr uint16_t BLACK = 0x0000;
constexpr uint16_t RED = 0xF800;
constexpr uint16_t GREEN = 0x07E0;
constexpr uint16_t BLUE = 0x001F;
constexpr uint16_t WHITE = 0xFFFF;

constexpr int PIN_SCL = 4;
constexpr int PIN_SDI = 5;
constexpr int PIN_CSX = 6;
constexpr int PIN_DCX = 7;
constexpr int PIN_RESX = 10;

uint8_t row[2 * WIDTH];

// Selects the chip with DCX low (command) or high (data). DCX is set before CSX falls,
// as the CO5300 Wokwi chip samples it there, so each phase gets its own CSX frame.
void select(bool data) {
  digitalWrite(PIN_DCX, data ? HIGH : LOW);
  digitalWrite(PIN_CSX, LOW);
}

void deselect() {
  digitalWrite(PIN_CSX, HIGH);
}

void command(uint8_t cmd, const uint8_t *params = nullptr, size_t len = 0) {
  select(false);
  SPI.write(cmd);
  deselect();
  if (len > 0) {
    select(true);
    SPI.writeBytes(params, len);
    deselect();
  }
}

void command(uint8_t cmd, uint8_t param) {
  command(cmd, &param, 1);
}

// Sets the window and starts RAMWR; send rows of 2 × w bytes, then call endPixels().
// The CO5300 needs even start and odd end addresses, so x, y, w and h must be even.
void beginPixels(uint16_t x, uint16_t y, uint16_t w, uint16_t h) {
  uint16_t x1 = x + w - 1, y1 = y + h - 1;
  uint8_t columns[] = {uint8_t(x >> 8), uint8_t(x), uint8_t(x1 >> 8), uint8_t(x1)};
  uint8_t rows[] = {uint8_t(y >> 8), uint8_t(y), uint8_t(y1 >> 8), uint8_t(y1)};
  command(CMD_CASET, columns, sizeof(columns));
  command(CMD_RASET, rows, sizeof(rows));
  command(CMD_RAMWR);
  select(true);
}

void endPixels() {
  deselect();
}

void setPixel(uint16_t i, uint16_t color) {
  row[2 * i] = color >> 8;
  row[2 * i + 1] = color;
}

void fillRect(uint16_t x, uint16_t y, uint16_t w, uint16_t h, uint16_t color) {
  for (uint16_t i = 0; i < w; i++) {
    setPixel(i, color);
  }

  beginPixels(x, y, w, h);
  for (uint16_t r = 0; r < h; r++) {
    SPI.writeBytes(row, 2 * w);
  }
  endPixels();
}

void setup() {
  Serial.begin(115200);

  pinMode(PIN_CSX, OUTPUT);
  pinMode(PIN_DCX, OUTPUT);
  pinMode(PIN_RESX, OUTPUT);
  digitalWrite(PIN_CSX, HIGH);
  digitalWrite(PIN_DCX, HIGH);

  SPI.begin(PIN_SCL, -1, PIN_SDI, -1);
  SPI.beginTransaction(SPISettings(40000000, MSBFIRST, SPI_MODE0));

  // Hardware reset.
  digitalWrite(PIN_RESX, LOW);
  delay(RESET_LOW_MS);
  digitalWrite(PIN_RESX, HIGH);
  delay(RESET_WAIT_MS);

  command(CMD_PAGESEL, PAGE_USER);
  command(CMD_SETSPIMODE, SPI_MODE_WRAM);
  command(CMD_SLPOUT);
  delay(SLEEP_OUT_TO_DISPLAY_ON_MS);
  command(CMD_COLMOD, COLMOD_RGB565);
  command(CMD_WRDISBV, BRIGHTNESS_MAX);
  command(CMD_DISPON);
  Serial.println("CO5300 init ok");

  fillRect(0, 0, WIDTH, HEIGHT, BLACK);

  // Four color bars on even coordinates, so the 2-pixel grid draws them exactly.
  const uint16_t bars[] = {RED, GREEN, BLUE, WHITE};
  for (int i = 0; i < 4; i++) {
    fillRect(20 + i * 96, 40, 80, 200, bars[i]);
  }

  // A red-to-blue gradient, with green rising from top to bottom.
  beginPixels(20, 280, 370, 64);
  for (uint16_t y = 0; y < 64; y++) {
    for (uint16_t x = 0; x < 370; x++) {
      uint16_t red = 31 * (369 - x) / 369;
      uint16_t blue = 31 * x / 369;
      setPixel(x, red << 11 | y << 5 | blue);
    }
    SPI.writeBytes(row, 2 * 370);
  }
  endPixels();

  Serial.println("CO5300 frame drawn");
}

void loop() {
  delay(1000);
}
