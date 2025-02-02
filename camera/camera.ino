#include "esp_camera.h"
#include <WiFi.h>
#include <AsyncTCP.h>
#include <ESPAsyncWebServer.h>
#include <LittleFS.h>
#include <ArduinoWebsockets.h>
#include <Preferences.h>

// Upload with "Huge APP" partition scheme for LittleFS to work
// Upload data/*.html using https://github.com/earlephilhower/arduino-littlefs-upload ([Ctrl] + [Shift] + [P], then "Upload LittleFS to Pico/ESP8266/ESP32")

// TODO: Only handle one connection at a time
// TODO: Investigate if TLS/encrypting images is too resource intensive

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

Preferences preferences;

// WiFi Details
const char* ssid = "VM9493530";
const char* password = "hnxZefs2abFifxad";

const char* websockets_server_host = "192.168.0.28"; //Enter server adress
const uint16_t websockets_server_port = 8080; // Enter server port
websockets::WebsocketsClient client;

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

void startAsyncCameraServer() {
  server.on("/", HTTP_GET, [](AsyncWebServerRequest *request) {
    // TODO: Redirect depending on if already set-up or not
    request->redirect("/setup.html");
  });

  server.on("/setup.html", HTTP_POST, [](AsyncWebServerRequest *request) {
    // TODO: Don't do anything if already set-up?
    String ssid_param = "";
    if (request->hasParam("ssid", true)) {
      ssid_param = request->getParam("ssid", true)->value();
    }

    String pass_param = "";
    if (request->hasParam("pass", true)) {
      pass_param = request->getParam("pass", true)->value();
    }

    if (!ssid_param.isEmpty() && !pass_param.isEmpty()) {
      Serial.println("Storing Wi-Fi details");
      preferences.begin("wifi", false);
      preferences.putString("ssid", ssid_param);
      preferences.putString("pass", pass_param);
      preferences.end();
    }

    request->redirect("/setup.html");
  });

  server.serveStatic("/setup.html", LittleFS, "/setup.html");

  server.begin();
}

void startWebSocketConnection() {
  bool connected = client.connect(websockets_server_host, websockets_server_port, "/");
    if (connected) {
        Serial.println("Connected!");
        client.send("Hello Server");
    } else {
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

  preferences.begin("wifi", false);
  String pref_ssid = preferences.getString("ssid", "");
  String pref_pass = preferences.getString("pass", "");
  preferences.end();

  if (!pref_ssid.isEmpty() && !pref_pass.isEmpty()) {
    Serial.println("Connecting to Wi-Fi using saved details");
    WiFi.begin(pref_ssid, pref_pass);
  } else {
    Serial.println("Connecting to Wi-Fi using default details");
    WiFi.begin(ssid, password);
  }

  WiFi.setSleep(false);

  while (WiFi.status() != WL_CONNECTED) {
    delay(500);
    Serial.print(".");

    if (digitalRead(FACTORY_RESET_PIN) == LOW) {
      Serial.println("FACTORY_RESET_PIN held LOW, resetting preferences");

      digitalWrite(LAMP_PIN, HIGH);

      preferences.begin("wifi", false);
      preferences.putString("ssid", "");
      preferences.putString("pass", "");
      preferences.end();

      delay(1000);

      digitalWrite(LAMP_PIN, LOW);
    }
  }
  Serial.println("");
  Serial.println("WiFi connected");

  esp_err_t err = setupCamera();
  if (err != ESP_OK) {
    Serial.printf("Camera init failed with error 0x%x", err);
    return;
  }

  startAsyncCameraServer();

  startWebSocketConnection();

  Serial.print("Camera Ready! Use 'http://");
  Serial.print(WiFi.localIP());
  Serial.println("' to connect");
}

void loop() {
  if (client.available()) {
    client.poll();
  }
  delay(500);
}
