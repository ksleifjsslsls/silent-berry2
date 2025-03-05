function dataLengthError(actual, required) {
  throw new Error(`Invalid data length! Required: ${required}, actual: ${actual}`);
}

function assertDataLength(actual, required) {
  if (actual !== required) {
    dataLengthError(actual, required);
  }
}

function assertArrayBuffer(reader) {
  if (reader instanceof Object && reader.toArrayBuffer instanceof Function) {
    reader = reader.toArrayBuffer();
  }
  if (!(reader instanceof ArrayBuffer)) {
    throw new Error("Provided value must be an ArrayBuffer or can be transformed into ArrayBuffer!");
  }
  return reader;
}

function verifyAndExtractOffsets(view, expectedFieldCount, compatible) {
  if (view.byteLength < 4) {
    dataLengthError(view.byteLength, ">4");
  }
  const requiredByteLength = view.getUint32(0, true);
  assertDataLength(view.byteLength, requiredByteLength);
  if (requiredByteLength === 4) {
    return [requiredByteLength];
  }
  if (requiredByteLength < 8) {
    dataLengthError(view.byteLength, ">8");
  }
  const firstOffset = view.getUint32(4, true);
  if (firstOffset % 4 !== 0 || firstOffset < 8) {
    throw new Error(`Invalid first offset: ${firstOffset}`);
  }
  const itemCount = firstOffset / 4 - 1;
  if (itemCount < expectedFieldCount) {
    throw new Error(`Item count not enough! Required: ${expectedFieldCount}, actual: ${itemCount}`);
  } else if ((!compatible) && itemCount > expectedFieldCount) {
    throw new Error(`Item count is more than required! Required: ${expectedFieldCount}, actual: ${itemCount}`);
  }
  if (requiredByteLength < firstOffset) {
    throw new Error(`First offset is larger than byte length: ${firstOffset}`);
  }
  const offsets = [];
  for (let i = 0; i < itemCount; i++) {
    const start = 4 + i * 4;
    offsets.push(view.getUint32(start, true));
  }
  offsets.push(requiredByteLength);
  for (let i = 0; i < offsets.length - 1; i++) {
    if (offsets[i] > offsets[i + 1]) {
      throw new Error(`Offset index ${i}: ${offsets[i]} is larger than offset index ${i + 1}: ${offsets[i + 1]}`);
    }
  }
  return offsets;
}

function serializeTable(buffers) {
  const itemCount = buffers.length;
  let totalSize = 4 * (itemCount + 1);
  const offsets = [];

  for (let i = 0; i < itemCount; i++) {
    offsets.push(totalSize);
    totalSize += buffers[i].byteLength;
  }

  const buffer = new ArrayBuffer(totalSize);
  const array = new Uint8Array(buffer);
  const view = new DataView(buffer);

  view.setUint32(0, totalSize, true);
  for (let i = 0; i < itemCount; i++) {
    view.setUint32(4 + i * 4, offsets[i], true);
    array.set(new Uint8Array(buffers[i]), offsets[i]);
  }
  return buffer;
}

export class WithdrawalSporeInfo {
  constructor(reader, { validate = true } = {}) {
    this.view = new DataView(assertArrayBuffer(reader));
    if (validate) {
      this.validate();
    }
  }

  getSporeCodeHash() {
    return new Byte32(this.view.buffer.slice(0, 0 + Byte32.size()), { validate: false });
  }

  getSporeLevel() {
    return this.view.getUint8(0 + Byte32.size());
  }

  getSporeId() {
    return new Byte32(this.view.buffer.slice(0 + Byte32.size() + 1, 0 + Byte32.size() + 1 + Byte32.size()), { validate: false });
  }

  getClusterId() {
    return new Byte32(this.view.buffer.slice(0 + Byte32.size() + 1 + Byte32.size(), 0 + Byte32.size() + 1 + Byte32.size() + Byte32.size()), { validate: false });
  }

  validate(compatible = false) {
    assertDataLength(this.view.byteLength, WithdrawalSporeInfo.size());
    this.getSporeCodeHash().validate(compatible);
    this.getSporeId().validate(compatible);
    this.getClusterId().validate(compatible);
  }
  static size() {
    return 0 + Byte32.size() + 1 + Byte32.size() + Byte32.size();
  }
}

export function SerializeWithdrawalSporeInfo(value) {
  const array = new Uint8Array(0 + Byte32.size() + 1 + Byte32.size() + Byte32.size());
  const view = new DataView(array.buffer);
  array.set(new Uint8Array(SerializeByte32(value.spore_code_hash)), 0);
  view.setUint8(0 + Byte32.size(), value.spore_level);
  array.set(new Uint8Array(SerializeByte32(value.spore_id)), 0 + Byte32.size() + 1);
  array.set(new Uint8Array(SerializeByte32(value.cluster_id)), 0 + Byte32.size() + 1 + Byte32.size());
  return array.buffer;
}

export class WithdrawalBuyer {
  constructor(reader, { validate = true } = {}) {
    this.view = new DataView(assertArrayBuffer(reader));
    if (validate) {
      this.validate();
    }
  }

  validate(compatible = false) {
    if (this.view.byteLength < 4) {
      assertDataLength(this.view.byteLength, ">4");
    }
    const t = this.view.getUint32(0, true);
    switch (t) {
      case 0:
        new WithdrawalSporeInfo(this.view.buffer.slice(4), { validate: false }).validate();
        break;
      case 1:
        new Byte32(this.view.buffer.slice(4), { validate: false }).validate();
        break;
      default:
        throw new Error(`Invalid type: ${t}`);
    }
  }

  unionType() {
    const t = this.view.getUint32(0, true);
    switch (t) {
      case 0:
        return "WithdrawalSporeInfo";
      case 1:
        return "Byte32";
      default:
        throw new Error(`Invalid type: ${t}`);
    }
  }

  value() {
    const t = this.view.getUint32(0, true);
    switch (t) {
      case 0:
        return new WithdrawalSporeInfo(this.view.buffer.slice(4), { validate: false });
      case 1:
        return new Byte32(this.view.buffer.slice(4), { validate: false });
      default:
        throw new Error(`Invalid type: ${t}`);
    }
  }
}

export function SerializeWithdrawalBuyer(value) {
  switch (value.type) {
    case "WithdrawalSporeInfo":
      {
        const itemBuffer = SerializeWithdrawalSporeInfo(value.value);
        const array = new Uint8Array(4 + itemBuffer.byteLength);
        const view = new DataView(array.buffer);
        view.setUint32(0, 0, true);
        array.set(new Uint8Array(itemBuffer), 4);
        return array.buffer;
      }
    case "Byte32":
      {
        const itemBuffer = SerializeByte32(value.value);
        const array = new Uint8Array(4 + itemBuffer.byteLength);
        const view = new DataView(array.buffer);
        view.setUint32(0, 1, true);
        array.set(new Uint8Array(itemBuffer), 4);
        return array.buffer;
      }
    default:
      throw new Error(`Invalid type: ${value.type}`);
  }
}

export class WithdrawalIntentData {
  constructor(reader, { validate = true } = {}) {
    this.view = new DataView(assertArrayBuffer(reader));
    if (validate) {
      this.validate();
    }
  }

  validate(compatible = false) {
    const offsets = verifyAndExtractOffsets(this.view, 0, true);
    new Byte32(this.view.buffer.slice(offsets[0], offsets[1]), { validate: false }).validate();
    new Byte32(this.view.buffer.slice(offsets[1], offsets[2]), { validate: false }).validate();
    new WithdrawalBuyer(this.view.buffer.slice(offsets[2], offsets[3]), { validate: false }).validate();
    new Uint64(this.view.buffer.slice(offsets[3], offsets[4]), { validate: false }).validate();
    new Byte32(this.view.buffer.slice(offsets[4], offsets[5]), { validate: false }).validate();
  }

  getXudtScriptHash() {
    const start = 4;
    const offset = this.view.getUint32(start, true);
    const offset_end = this.view.getUint32(start + 4, true);
    return new Byte32(this.view.buffer.slice(offset, offset_end), { validate: false });
  }

  getXudtLockScriptHash() {
    const start = 8;
    const offset = this.view.getUint32(start, true);
    const offset_end = this.view.getUint32(start + 4, true);
    return new Byte32(this.view.buffer.slice(offset, offset_end), { validate: false });
  }

  getBuyer() {
    const start = 12;
    const offset = this.view.getUint32(start, true);
    const offset_end = this.view.getUint32(start + 4, true);
    return new WithdrawalBuyer(this.view.buffer.slice(offset, offset_end), { validate: false });
  }

  getExpireSince() {
    const start = 16;
    const offset = this.view.getUint32(start, true);
    const offset_end = this.view.getUint32(start + 4, true);
    return new Uint64(this.view.buffer.slice(offset, offset_end), { validate: false });
  }

  getOwnerScriptHash() {
    const start = 20;
    const offset = this.view.getUint32(start, true);
    const offset_end = this.view.byteLength;
    return new Byte32(this.view.buffer.slice(offset, offset_end), { validate: false });
  }
}

export function SerializeWithdrawalIntentData(value) {
  const buffers = [];
  buffers.push(SerializeByte32(value.xudt_script_hash));
  buffers.push(SerializeByte32(value.xudt_lock_script_hash));
  buffers.push(SerializeWithdrawalBuyer(value.buyer));
  buffers.push(SerializeUint64(value.expire_since));
  buffers.push(SerializeByte32(value.owner_script_hash));
  return serializeTable(buffers);
}

export class Uint64 {
  constructor(reader, { validate = true } = {}) {
    this.view = new DataView(assertArrayBuffer(reader));
    if (validate) {
      this.validate();
    }
  }

  validate(compatible = false) {
    assertDataLength(this.view.byteLength, 8);
  }

  indexAt(i) {
    return this.view.getUint8(i);
  }

  raw() {
    return this.view.buffer;
  }

  static size() {
    return 8;
  }
}

export function SerializeUint64(value) {
  const buffer = assertArrayBuffer(value);
  assertDataLength(buffer.byteLength, 8);
  return buffer;
}

export class Uint128 {
  constructor(reader, { validate = true } = {}) {
    this.view = new DataView(assertArrayBuffer(reader));
    if (validate) {
      this.validate();
    }
  }

  validate(compatible = false) {
    assertDataLength(this.view.byteLength, 16);
  }

  indexAt(i) {
    return this.view.getUint8(i);
  }

  raw() {
    return this.view.buffer;
  }

  static size() {
    return 16;
  }
}

export function SerializeUint128(value) {
  const buffer = assertArrayBuffer(value);
  assertDataLength(buffer.byteLength, 16);
  return buffer;
}

export class Byte32 {
  constructor(reader, { validate = true } = {}) {
    this.view = new DataView(assertArrayBuffer(reader));
    if (validate) {
      this.validate();
    }
  }

  validate(compatible = false) {
    assertDataLength(this.view.byteLength, 32);
  }

  indexAt(i) {
    return this.view.getUint8(i);
  }

  raw() {
    return this.view.buffer;
  }

  static size() {
    return 32;
  }
}

export function SerializeByte32(value) {
  const buffer = assertArrayBuffer(value);
  assertDataLength(buffer.byteLength, 32);
  return buffer;
}

export class Bytes {
  constructor(reader, { validate = true } = {}) {
    this.view = new DataView(assertArrayBuffer(reader));
    if (validate) {
      this.validate();
    }
  }

  validate(compatible = false) {
    if (this.view.byteLength < 4) {
      dataLengthError(this.view.byteLength, ">4")
    }
    const requiredByteLength = this.length() + 4;
    assertDataLength(this.view.byteLength, requiredByteLength);
  }

  raw() {
    return this.view.buffer.slice(4);
  }

  indexAt(i) {
    return this.view.getUint8(4 + i);
  }

  length() {
    return this.view.getUint32(0, true);
  }
}

export function SerializeBytes(value) {
  const item = assertArrayBuffer(value);
  const array = new Uint8Array(4 + item.byteLength);
  (new DataView(array.buffer)).setUint32(0, item.byteLength, true);
  array.set(new Uint8Array(item), 4);
  return array.buffer;
}

export class BytesOpt {
  constructor(reader, { validate = true } = {}) {
    this.view = new DataView(assertArrayBuffer(reader));
    if (validate) {
      this.validate();
    }
  }

  validate(compatible = false) {
    if (this.hasValue()) {
      this.value().validate(compatible);
    }
  }

  value() {
    return new Bytes(this.view.buffer, { validate: false });
  }

  hasValue() {
    return this.view.byteLength > 0;
  }
}

export function SerializeBytesOpt(value) {
  if (value) {
    return SerializeBytes(value);
  } else {
    return new ArrayBuffer(0);
  }
}
