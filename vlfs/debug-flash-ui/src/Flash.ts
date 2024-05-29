export const FLASH_SIZE = 262144 * 256;

export class Flash {
  data: Uint8Array = new Uint8Array(FLASH_SIZE);
  constructor() {}

  getByte(address: number): number {
    return this.data[address];
  }

  eraseSector4Kib(address: number) {
    for (let i = 0; i < 4096; i++) {
      this.data[address + i] = 0xff;
    }
  }

  eraseBlock32Kib(address: number) {
    for (let i = 0; i < 32768; i++) {
      this.data[address + i] = 0xff;
    }
  }

  eraseBlock64Kib(address: number) {
    for (let i = 0; i < 65536; i++) {
      this.data[address + i] = 0xff;
    }
  }

  read(address: number, length:number): Uint8Array {
    return this.data.slice(address, address + length);
  }

  write256b(address: number, data: Uint8Array) {
    this.data.set(data, address);
  }
}
