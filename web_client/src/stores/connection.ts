import { defineStore, acceptHMRUpdate } from 'pinia';
import { io, type Socket } from 'socket.io-client';

export interface Vec3 {
  x: number;
  y: number;
  z: number;
}

export interface Quat {
  x: number;
  y: number;
  z: number;
  w: number;
}

export interface PoseInput {
  position: Vec3;
  orientation: Quat;
}

export interface ButtonsInput {
  trigger: number;
  squeeze: number;
  thumbstick_x: number;
  thumbstick_y: number;
  thumbstick_click: boolean;
  primary_click: boolean;
  secondary_click: boolean;
  menu_click: boolean;
}

export interface HandInput {
  pose: PoseInput;
  buttons: ButtonsInput;
}

export interface DeviceInput {
  head: PoseInput;
  leftHand: HandInput;
  rightHand: HandInput;
}

interface ConnectionStore {
  isConnected: boolean;
  isConnecting: boolean;
  address: string | null;
  socket: Socket | null;
  // Keyed by "<swapchain_id>:<layer>" — a swapchain can be a plain 2D image
  // (layer always 0, one swapchain per eye) or an array image with one layer
  // per eye (one swapchain, layer 0/1), so both need to be told apart.
  frames: Record<string, string>;
}

export const useConnection = defineStore('connection', {
  state: (): ConnectionStore => ({
    isConnected: false,
    isConnecting: false,
    address: null,
    socket: null,
    frames: {},
  }),

  getters: {},

  actions: {
    connect(address: string) {
      if (this.isConnected) {
        return;
      }

      this.isConnecting = true;
      this.address = address;
      this.socket = io(address);

      return new Promise((resolve, reject) => {
        if (this.socket == null) {
          return;
        }

        this.socket.on('connect', () => {
          this.isConnected = true;
          this.isConnecting = false;
          resolve(true);
        });

        this.socket.on('connect_error', (err) => {
          console.log('connect_error');
          if (this.socket?.active) {
            this.socket?.disconnect();
          }

          this.isConnecting = true;
          this.isConnected = false;
          reject(err);
        });

        this.socket.on('disconnect', () => {
          console.log('disconnected');
          this.isConnected = false;
          this.isConnecting = false;
        });

        this.socket.on('message-back', (data: unknown) => {
          console.log('message-back', data);
        });

        this.socket.on(
          'frame',
          (data: { number: number; swapchain_id: number; layer: number; jpeg_b64: string }) => {
            this.frames[`${data.swapchain_id}:${data.layer}`] =
              `data:image/jpeg;base64,${data.jpeg_b64}`;
          },
        );
      });
    },

    ping() {
      this.socket?.emit('message', 'test msg');
    },

    sendInput(input: DeviceInput) {
      this.socket?.emit('input', {
        head: input.head,
        left_hand: input.leftHand,
        right_hand: input.rightHand,
      });
    },
  },
});

if (import.meta.hot) {
  import.meta.hot.accept(acceptHMRUpdate(useConnection, import.meta.hot));
}
