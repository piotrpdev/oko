#include "esp_camera.h"
#include <WiFi.h>
#include <AsyncTCP.h>
#include <ESPAsyncWebServer.h>
#include <LittleFS.h>
#include <ArduinoWebsockets.h>
#include <Preferences.h>
#include <DNSServer.h>

// Default Windows 11 CH340 drivers might cause issues https://forum.arduino.cc/t/a-device-attached-to-the-system-is-not-functioning/1165392/9
// Upload with "Huge APP" partition scheme for LittleFS to work
// Upload data/*.html using https://github.com/earlephilhower/arduino-littlefs-upload ([Ctrl] + [Shift] + [P], then "Upload LittleFS to Pico/ESP8266/ESP32")
// Prefs are saved across RST. Hit RST then hold IO0 (don't hold both) to reset Wi-Fi (flash will fire).

// TODO: Only handle one connection at a time
// TODO: Adjust lamp brightness (setup ledc)
// TODO: Display possible networks to connect to
// TODO: After setup, display success message/tell user to restart
// TODO: WSL / TLS / Investigate if TLS/encrypting images is too resource intensive

// CAMERA_MODEL_AI_THINKER pins
#define PWDN_GPIO_NUM     32
#define RESET_GPIO_NUM    -1
#define XCLK_GPIO_NUM      0
#define SIOD_GPIO_NUM     26
#define SIOC_GPIO_NUM     27

#define Y9_GPIO_NUM       35
#define Y8_GPIO_NUM       34
#define Y7_GPIO_NUM       39
#define Y6_GPIO_NUM       36
#define Y5_GPIO_NUM       21
#define Y4_GPIO_NUM       19
#define Y3_GPIO_NUM       18
#define Y2_GPIO_NUM        5
#define VSYNC_GPIO_NUM    25
#define HREF_GPIO_NUM     23
#define PCLK_GPIO_NUM     22

#define LED_PIN           33
#define LAMP_PIN           4

#define FACTORY_RESET_PIN  0

static AsyncWebServer server(80);

static Preferences preferences;

// AP WiFi Details
static DNSServer dnsServer;
const char* ap_ssid = "ESP32-CAM Setup";
const char* ap_password = NULL;
static bool needs_setup = true;

static websockets::WebsocketsClient client;

#define CAPTURE_INTERVAL 100
unsigned long previousMillis = 0;

esp_err_t setupCamera() {
  camera_config_t config;
  config.ledc_channel = LEDC_CHANNEL_0;
  config.ledc_timer = LEDC_TIMER_0;
  config.pin_d0 = Y2_GPIO_NUM;
  config.pin_d1 = Y3_GPIO_NUM;
  config.pin_d2 = Y4_GPIO_NUM;
  config.pin_d3 = Y5_GPIO_NUM;
  config.pin_d4 = Y6_GPIO_NUM;
  config.pin_d5 = Y7_GPIO_NUM;
  config.pin_d6 = Y8_GPIO_NUM;
  config.pin_d7 = Y9_GPIO_NUM;
  config.pin_xclk = XCLK_GPIO_NUM;
  config.pin_pclk = PCLK_GPIO_NUM;
  config.pin_vsync = VSYNC_GPIO_NUM;
  config.pin_href = HREF_GPIO_NUM;
  config.pin_sccb_sda = SIOD_GPIO_NUM;
  config.pin_sccb_scl = SIOC_GPIO_NUM;
  config.pin_pwdn = PWDN_GPIO_NUM;
  config.pin_reset = RESET_GPIO_NUM;
  config.xclk_freq_hz = 8 * 1000000;
  config.pixel_format = PIXFORMAT_JPEG; // for streaming
  config.frame_size = FRAMESIZE_SVGA;
  config.jpeg_quality = 12;
  config.grab_mode = CAMERA_GRAB_LATEST;
  config.fb_location = CAMERA_FB_IN_PSRAM;
  config.jpeg_quality = 12;
  config.fb_count = 2;

  // camera init
  return esp_camera_init(&config);
}

class CaptiveRequestHandler : public AsyncWebHandler {
public:
  bool canHandle(AsyncWebServerRequest *request) override {
    String url = request->url();
    return url != "/setup.html" && url != "/setup.html#success" && url != "/setup.html#error";
  }

  void handleRequest(AsyncWebServerRequest *request) {
    request->redirect("http://" + WiFi.softAPIP().toString() + "/setup.html");
  }
};

void startAsyncCameraServer() {
  server.addHandler(new CaptiveRequestHandler()).setFilter(ON_AP_FILTER);

  server.on("/", HTTP_GET, [](AsyncWebServerRequest *request) {
    // TODO: Redirect depending on if already set-up or not
    request->redirect("/setup.html");
  });

  server.on("/setup.html", HTTP_POST, [](AsyncWebServerRequest *request) {
    // TODO: Don't do anything if already set-up?

    // https://paginas.fe.up.pt/~jaime/0506/SSR/802.11i-2004.pdf
    // SSID is basically arbitrary data, spec says pretty much anything is allowed (hopefully no exploit here)
    String ssid_param = "";
    if (request->hasParam("ssid", true)) {
      ssid_param = request->getParam("ssid", true)->value();
      ssid_param.trim();
    }
    bool ssid_param_valid = !ssid_param.isEmpty() && ssid_param.length() <= 32;

    String pass_param = "";
    if (request->hasParam("pass", true)) {
      pass_param = request->getParam("pass", true)->value();
      pass_param.trim();
    }
    bool pass_param_valid = !pass_param.isEmpty() && pass_param.length() >= 8 && pass_param.length() <= 63;

    // Oko IP e.g. 192.168.0.28:8080
    String oko_param = "";
    if (request->hasParam("oko", true)) {
      oko_param = request->getParam("oko", true)->value();
      oko_param.trim();
    }
    bool oko_param_valid = false;
    // X.X.X.X:XXXXX -> XXX.XXX.XXX.XXX:XXXXX
    // TODO: This code probably isn't fully safe, check for buffer/integer overflows
    if (oko_param.length() >= 9 && oko_param.length() <= 21) {
      int ip[5];
      int parsed_value_count = sscanf(oko_param.c_str(), "%u.%u.%u.%u:%u", &ip[0], &ip[1], &ip[2], &ip[3], &ip[4]);
      int valid_count = 0;
      if (parsed_value_count == 5) {
        for (int i = 0; i < 5; i++) {
          if (i < 4 && (ip[i] >= 0 && ip[i] <= 255)) {
            valid_count++;
          } else if (i == 4 && (ip[i] > 0 && ip[i] <= 65535)) {
            valid_count++;
          }
        }
      }
      if (valid_count == 5) {
        oko_param_valid = true;
      }
    }

    bool valid = ssid_param_valid && pass_param_valid && oko_param_valid;
    if (valid) {
      Serial.println("Storing Wi-Fi details");
      preferences.begin("prefs", false);
      preferences.putString("ssid", ssid_param);
      preferences.putString("pass", pass_param);
      preferences.putString("oko", oko_param);
      preferences.end();

      request->redirect("/setup.html#success");

      Serial.println("Restarting with new Wi-Fi details");
      ESP.restart();
    } else {
      Serial.println("Invalid setup form submitted");
      request->redirect("/setup.html#error");
    }
  });

  server.serveStatic("/setup.html", LittleFS, "/setup.html");

  server.begin();
}

void startWebSocketConnection() {
  preferences.begin("prefs", false);
  String pref_oko = preferences.getString("oko", "");
  preferences.end();

  if (pref_oko.isEmpty()) {
    Serial.println("Oko IP address not set, skipping WebSocket setup");
    return;
  }

  bool connected = client.connect("ws://" + pref_oko + "/api/ws");
  if (connected) {
      Serial.println("Connected!");
      client.send("camera_any_port");
      sleep(2);

      camera_fb_t *fb = NULL;

      // TODO: Move into loop() ?
      while (true) {
        unsigned long currentMillis = millis();

        if (currentMillis - previousMillis >= CAPTURE_INTERVAL) {
          previousMillis = currentMillis;

          fb = esp_camera_fb_get();
          if (!fb) {
            Serial.println("Camera capture failed");
            continue;
          }

          // https://github.com/gilmaimon/ArduinoWebsockets/issues/16
          // TODO: try streaming? e.g. .streamBinary()
          client.sendBinary((const char *)fb->buf, fb->len);

          esp_camera_fb_return(fb);
          fb = NULL;
        }
      }
  } else {
      // Remmember to check firewall
      Serial.println("Not Connected!");
  }
    
  // run callback when messages are received
  client.onMessage([&](websockets::WebsocketsMessage message){
      Serial.print("Got Message: ");
      Serial.println(message.data());
  });
}

void setup() {
  Serial.begin(115200);
  Serial.setDebugOutput(true);
  Serial.println();

  pinMode(FACTORY_RESET_PIN, INPUT);
  pinMode(LAMP_PIN, OUTPUT);

  LittleFS.begin(true);

  preferences.begin("prefs", false);
  String pref_ssid = preferences.getString("ssid", "");
  String pref_pass = preferences.getString("pass", "");
  preferences.end();

  needs_setup = pref_ssid.isEmpty() || pref_pass.isEmpty();

  if (needs_setup) {
    Serial.println("Creating AP for initial user setup");
    WiFi.mode(WIFI_AP);
    if (!WiFi.softAP(ap_ssid, ap_password)) {
      Serial.println("Soft AP creation failed.");
      return;
    }
    WiFi.setSleep(false);
    dnsServer.start(53, "*", WiFi.softAPIP());
    Serial.println("");
    Serial.println("AP created");
    Serial.print("IP address: ");
    Serial.println(WiFi.softAPIP());
  } else {
    Serial.println("Connecting to Wi-Fi using saved details");
    WiFi.mode(WIFI_STA);
    WiFi.begin(pref_ssid, pref_pass);
    WiFi.setSleep(false);

    while (WiFi.status() != WL_CONNECTED) {
      delay(500);
      Serial.print(".");

      // GPI0 gets used as CLK later on, so can only use it before esp_camera_init() is called
      if (digitalRead(FACTORY_RESET_PIN) == LOW) {
        Serial.println("FACTORY_RESET_PIN held LOW, resetting preferences");

        digitalWrite(LAMP_PIN, HIGH);

        preferences.begin("prefs", false);
        preferences.putString("ssid", "");
        preferences.putString("pass", "");
        preferences.end();

        delay(1000);

        digitalWrite(LAMP_PIN, LOW);

        Serial.println("Preferences reset, use RST to restart device");
      }
    }
    Serial.println("");
    Serial.println("WiFi connected");
    Serial.print("IP address: ");
    Serial.println(WiFi.localIP());
  }

  esp_err_t err = setupCamera();
  if (err != ESP_OK) {
    Serial.printf("Camera init failed with error 0x%x", err);
    return;
  }

  startAsyncCameraServer();

  if (!needs_setup) {
    startWebSocketConnection();
  }
}

void loop() {
  if (needs_setup) {
    dnsServer.processNextRequest();
  } else {
    if (client.available()) {
      client.poll();
    }
    delay(500);
  }
}
