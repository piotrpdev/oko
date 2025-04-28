export type User = {
  user_id: number;
  username: string;
  password_hash: string;
  created_at: Array<number>;
};

export type MdnsCamera = {
  hostname: string;
  socket_address: string;
};

export function isMdnsCamera(obj: unknown): obj is MdnsCamera {
  return (
    obj instanceof Object &&
    "hostname" in obj &&
    "socket_address" in obj &&
    typeof obj.hostname === "string" &&
    typeof obj.socket_address === "string"
  );
}

export type Camera = {
  camera_id: number;
  camera_name: string;
  ip_address: string;
  can_control: boolean;
  can_view: boolean;
};

export type UserAndCameras = {
  user: User;
  cameras: Array<Camera>;
};

export type VideoCameraView = {
  video_id: number;
  camera_id: number;
  camera_name: string;
  file_path: string;
  file_size: number;
};

export type CameraPermission = {
  permission_id: number;
  camera_id: number;
  user_id: number;
  username: string;
  can_view: boolean;
  can_control: boolean;
};

export type CameraSetting = {
  camera_id: number;
  flashlight_enabled: boolean;
  framerate: number;
  last_modified: Array<number>;
  modified_by: number;
  resolution: string;
  setting_id: number;
};

export type ImageContainer = {
  camera_id: number;
  timestamp: number;
  image_bytes: Array<number>;
};

export function isImageContainer(obj: unknown): obj is ImageContainer {
  return (
    obj instanceof Object &&
    "camera_id" in obj &&
    "timestamp" in obj &&
    "image_bytes" in obj &&
    typeof obj.camera_id === "number" &&
    typeof obj.timestamp === "number" &&
    Array.isArray(obj.image_bytes)
  );
}

export type CameraListChangeType = "Added" | "Removed" | "Updated";

export type CameraListChangeCamera = {
  camera_id: number;
};

export type CameraListChange = {
  [key in CameraListChangeType]?: CameraListChangeCamera;
};

export function isCameraListChange(obj: unknown): obj is CameraListChange {
  return (
    obj instanceof Object &&
    (("Added" in obj &&
      obj.Added instanceof Object &&
      "camera_id" in obj.Added &&
      typeof obj.Added.camera_id === "number") ||
      ("Removed" in obj &&
        obj.Removed instanceof Object &&
        "camera_id" in obj.Removed &&
        typeof obj.Removed.camera_id === "number") ||
      ("Updated" in obj &&
        obj.Updated instanceof Object &&
        "camera_id" in obj.Updated &&
        typeof obj.Updated.camera_id === "number"))
  );
}
