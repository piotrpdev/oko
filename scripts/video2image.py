# Code taken from https://github.com/Wayan123/convert-video2image-and-image2video-using-python/blob/3886bf02af4b3c31d566b95ff7af1c9ad2ef7bc8/video2image.py
# Related article: https://medium.com/@wayandadangunsri/converting-video-to-images-using-python-and-opencv-72b2ea66a692
# "Don’t hesitate to make adjustments and employ this code according to your video-to-image conversion needs."
import sys
import time

import cv2
import websocket

ws = websocket.WebSocket()

# Function to extract frames from a video until reaching the desired frame count
def extract_frames(video_file):
    cap = cv2.VideoCapture(video_file)

    while True:
        ret, frame = cap.read()

        if not ret:
            break

        time.sleep(1)

        ws.send(frame.tobytes(), websocket.ABNF.OPCODE_BINARY)

    cap.release()

if __name__ == "__main__":
    video_file = f"{sys.argv[1]}.mp4"

    ws.connect("ws://localhost:3000/ws")

    extract_frames(video_file)

    ws.close()
