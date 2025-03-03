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

export class Script {
  constructor(reader, { validate = true } = {}) {
    this.view = new DataView(assertArrayBuffer(reader));
    if (validate) {
      this.validate();
    }
  }

  validate(compatible = false) {
    const offsets = verifyAndExtractOffsets(this.view, 0, true);
    new Byte32(this.view.buffer.slice(offsets[0], offsets[1]), { validate: false }).validate();
    if (offsets[2] - offsets[1] !== 1) {
      throw new Error(`Invalid offset for hash_type: ${offsets[1]} - ${offsets[2]}`)
    }
    new Bytes(this.view.buffer.slice(offsets[2], offsets[3]), { validate: false }).validate();
  }

  getCodeHash() {
    const start = 4;
    const offset = this.view.getUint32(start, true);
    const offset_end = this.view.getUint32(start + 4, true);
    return new Byte32(this.view.buffer.slice(offset, offset_end), { validate: false });
  }

  getHashType() {
    const start = 8;
    const offset = this.view.getUint32(start, true);
    const offset_end = this.view.getUint32(start + 4, true);
    return new DataView(this.view.buffer.slice(offset, offset_end)).getUint8(0);
  }

  getArgs() {
    const start = 12;
    const offset = this.view.getUint32(start, true);
    const offset_end = this.view.byteLength;
    return new Bytes(this.view.buffer.slice(offset, offset_end), { validate: false });
  }
}

export function SerializeScript(value) {
  const buffers = [];
  buffers.push(SerializeByte32(value.code_hash));
  const hashTypeView = new DataView(new ArrayBuffer(1));
  hashTypeView.setUint8(0, value.hash_type);
  buffers.push(hashTypeView.buffer)
  buffers.push(SerializeBytes(value.args));
  return serializeTable(buffers);
}

export class Address {
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
      new Script(this.view.buffer.slice(4), { validate: false }).validate();
      break;
    default:
      throw new Error(`Invalid type: ${t}`);
    }
  }

  unionType() {
    const t = this.view.getUint32(0, true);
    switch (t) {
    case 0:
      return "Script";
    default:
      throw new Error(`Invalid type: ${t}`);
    }
  }

  value() {
    const t = this.view.getUint32(0, true);
    switch (t) {
    case 0:
      return new Script(this.view.buffer.slice(4), { validate: false });
    default:
      throw new Error(`Invalid type: ${t}`);
    }
  }
}

export function SerializeAddress(value) {
  switch (value.type) {
  case "Script":
    {
      const itemBuffer = SerializeScript(value.value);
      const array = new Uint8Array(4 + itemBuffer.byteLength);
      const view = new DataView(array.buffer);
      view.setUint32(0, 0, true);
      array.set(new Uint8Array(itemBuffer), 4);
      return array.buffer;
    }
  default:
    throw new Error(`Invalid type: ${value.type}`);
  }
}

export class MintSpore {
  constructor(reader, { validate = true } = {}) {
    this.view = new DataView(assertArrayBuffer(reader));
    if (validate) {
      this.validate();
    }
  }

  validate(compatible = false) {
    const offsets = verifyAndExtractOffsets(this.view, 0, true);
    new Byte32(this.view.buffer.slice(offsets[0], offsets[1]), { validate: false }).validate();
    new Address(this.view.buffer.slice(offsets[1], offsets[2]), { validate: false }).validate();
    new Byte32(this.view.buffer.slice(offsets[2], offsets[3]), { validate: false }).validate();
  }

  getSporeId() {
    const start = 4;
    const offset = this.view.getUint32(start, true);
    const offset_end = this.view.getUint32(start + 4, true);
    return new Byte32(this.view.buffer.slice(offset, offset_end), { validate: false });
  }

  getTo() {
    const start = 8;
    const offset = this.view.getUint32(start, true);
    const offset_end = this.view.getUint32(start + 4, true);
    return new Address(this.view.buffer.slice(offset, offset_end), { validate: false });
  }

  getDataHash() {
    const start = 12;
    const offset = this.view.getUint32(start, true);
    const offset_end = this.view.byteLength;
    return new Byte32(this.view.buffer.slice(offset, offset_end), { validate: false });
  }
}

export function SerializeMintSpore(value) {
  const buffers = [];
  buffers.push(SerializeByte32(value.spore_id));
  buffers.push(SerializeAddress(value.to));
  buffers.push(SerializeByte32(value.data_hash));
  return serializeTable(buffers);
}

export class TransferSpore {
  constructor(reader, { validate = true } = {}) {
    this.view = new DataView(assertArrayBuffer(reader));
    if (validate) {
      this.validate();
    }
  }

  validate(compatible = false) {
    const offsets = verifyAndExtractOffsets(this.view, 0, true);
    new Byte32(this.view.buffer.slice(offsets[0], offsets[1]), { validate: false }).validate();
    new Address(this.view.buffer.slice(offsets[1], offsets[2]), { validate: false }).validate();
    new Address(this.view.buffer.slice(offsets[2], offsets[3]), { validate: false }).validate();
  }

  getSporeId() {
    const start = 4;
    const offset = this.view.getUint32(start, true);
    const offset_end = this.view.getUint32(start + 4, true);
    return new Byte32(this.view.buffer.slice(offset, offset_end), { validate: false });
  }

  getFrom() {
    const start = 8;
    const offset = this.view.getUint32(start, true);
    const offset_end = this.view.getUint32(start + 4, true);
    return new Address(this.view.buffer.slice(offset, offset_end), { validate: false });
  }

  getTo() {
    const start = 12;
    const offset = this.view.getUint32(start, true);
    const offset_end = this.view.byteLength;
    return new Address(this.view.buffer.slice(offset, offset_end), { validate: false });
  }
}

export function SerializeTransferSpore(value) {
  const buffers = [];
  buffers.push(SerializeByte32(value.spore_id));
  buffers.push(SerializeAddress(value.from));
  buffers.push(SerializeAddress(value.to));
  return serializeTable(buffers);
}

export class BurnSpore {
  constructor(reader, { validate = true } = {}) {
    this.view = new DataView(assertArrayBuffer(reader));
    if (validate) {
      this.validate();
    }
  }

  validate(compatible = false) {
    const offsets = verifyAndExtractOffsets(this.view, 0, true);
    new Byte32(this.view.buffer.slice(offsets[0], offsets[1]), { validate: false }).validate();
    new Address(this.view.buffer.slice(offsets[1], offsets[2]), { validate: false }).validate();
  }

  getSporeId() {
    const start = 4;
    const offset = this.view.getUint32(start, true);
    const offset_end = this.view.getUint32(start + 4, true);
    return new Byte32(this.view.buffer.slice(offset, offset_end), { validate: false });
  }

  getFrom() {
    const start = 8;
    const offset = this.view.getUint32(start, true);
    const offset_end = this.view.byteLength;
    return new Address(this.view.buffer.slice(offset, offset_end), { validate: false });
  }
}

export function SerializeBurnSpore(value) {
  const buffers = [];
  buffers.push(SerializeByte32(value.spore_id));
  buffers.push(SerializeAddress(value.from));
  return serializeTable(buffers);
}

export class MintCluster {
  constructor(reader, { validate = true } = {}) {
    this.view = new DataView(assertArrayBuffer(reader));
    if (validate) {
      this.validate();
    }
  }

  validate(compatible = false) {
    const offsets = verifyAndExtractOffsets(this.view, 0, true);
    new Byte32(this.view.buffer.slice(offsets[0], offsets[1]), { validate: false }).validate();
    new Address(this.view.buffer.slice(offsets[1], offsets[2]), { validate: false }).validate();
    new Byte32(this.view.buffer.slice(offsets[2], offsets[3]), { validate: false }).validate();
  }

  getClusterId() {
    const start = 4;
    const offset = this.view.getUint32(start, true);
    const offset_end = this.view.getUint32(start + 4, true);
    return new Byte32(this.view.buffer.slice(offset, offset_end), { validate: false });
  }

  getTo() {
    const start = 8;
    const offset = this.view.getUint32(start, true);
    const offset_end = this.view.getUint32(start + 4, true);
    return new Address(this.view.buffer.slice(offset, offset_end), { validate: false });
  }

  getDataHash() {
    const start = 12;
    const offset = this.view.getUint32(start, true);
    const offset_end = this.view.byteLength;
    return new Byte32(this.view.buffer.slice(offset, offset_end), { validate: false });
  }
}

export function SerializeMintCluster(value) {
  const buffers = [];
  buffers.push(SerializeByte32(value.cluster_id));
  buffers.push(SerializeAddress(value.to));
  buffers.push(SerializeByte32(value.data_hash));
  return serializeTable(buffers);
}

export class TransferCluster {
  constructor(reader, { validate = true } = {}) {
    this.view = new DataView(assertArrayBuffer(reader));
    if (validate) {
      this.validate();
    }
  }

  validate(compatible = false) {
    const offsets = verifyAndExtractOffsets(this.view, 0, true);
    new Byte32(this.view.buffer.slice(offsets[0], offsets[1]), { validate: false }).validate();
    new Address(this.view.buffer.slice(offsets[1], offsets[2]), { validate: false }).validate();
    new Address(this.view.buffer.slice(offsets[2], offsets[3]), { validate: false }).validate();
  }

  getClusterId() {
    const start = 4;
    const offset = this.view.getUint32(start, true);
    const offset_end = this.view.getUint32(start + 4, true);
    return new Byte32(this.view.buffer.slice(offset, offset_end), { validate: false });
  }

  getFrom() {
    const start = 8;
    const offset = this.view.getUint32(start, true);
    const offset_end = this.view.getUint32(start + 4, true);
    return new Address(this.view.buffer.slice(offset, offset_end), { validate: false });
  }

  getTo() {
    const start = 12;
    const offset = this.view.getUint32(start, true);
    const offset_end = this.view.byteLength;
    return new Address(this.view.buffer.slice(offset, offset_end), { validate: false });
  }
}

export function SerializeTransferCluster(value) {
  const buffers = [];
  buffers.push(SerializeByte32(value.cluster_id));
  buffers.push(SerializeAddress(value.from));
  buffers.push(SerializeAddress(value.to));
  return serializeTable(buffers);
}

export class MintProxy {
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
    new Address(this.view.buffer.slice(offsets[2], offsets[3]), { validate: false }).validate();
  }

  getClusterId() {
    const start = 4;
    const offset = this.view.getUint32(start, true);
    const offset_end = this.view.getUint32(start + 4, true);
    return new Byte32(this.view.buffer.slice(offset, offset_end), { validate: false });
  }

  getProxyId() {
    const start = 8;
    const offset = this.view.getUint32(start, true);
    const offset_end = this.view.getUint32(start + 4, true);
    return new Byte32(this.view.buffer.slice(offset, offset_end), { validate: false });
  }

  getTo() {
    const start = 12;
    const offset = this.view.getUint32(start, true);
    const offset_end = this.view.byteLength;
    return new Address(this.view.buffer.slice(offset, offset_end), { validate: false });
  }
}

export function SerializeMintProxy(value) {
  const buffers = [];
  buffers.push(SerializeByte32(value.cluster_id));
  buffers.push(SerializeByte32(value.proxy_id));
  buffers.push(SerializeAddress(value.to));
  return serializeTable(buffers);
}

export class TransferProxy {
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
    new Address(this.view.buffer.slice(offsets[2], offsets[3]), { validate: false }).validate();
    new Address(this.view.buffer.slice(offsets[3], offsets[4]), { validate: false }).validate();
  }

  getClusterId() {
    const start = 4;
    const offset = this.view.getUint32(start, true);
    const offset_end = this.view.getUint32(start + 4, true);
    return new Byte32(this.view.buffer.slice(offset, offset_end), { validate: false });
  }

  getProxyId() {
    const start = 8;
    const offset = this.view.getUint32(start, true);
    const offset_end = this.view.getUint32(start + 4, true);
    return new Byte32(this.view.buffer.slice(offset, offset_end), { validate: false });
  }

  getFrom() {
    const start = 12;
    const offset = this.view.getUint32(start, true);
    const offset_end = this.view.getUint32(start + 4, true);
    return new Address(this.view.buffer.slice(offset, offset_end), { validate: false });
  }

  getTo() {
    const start = 16;
    const offset = this.view.getUint32(start, true);
    const offset_end = this.view.byteLength;
    return new Address(this.view.buffer.slice(offset, offset_end), { validate: false });
  }
}

export function SerializeTransferProxy(value) {
  const buffers = [];
  buffers.push(SerializeByte32(value.cluster_id));
  buffers.push(SerializeByte32(value.proxy_id));
  buffers.push(SerializeAddress(value.from));
  buffers.push(SerializeAddress(value.to));
  return serializeTable(buffers);
}

export class BurnProxy {
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
    new Address(this.view.buffer.slice(offsets[2], offsets[3]), { validate: false }).validate();
  }

  getClusterId() {
    const start = 4;
    const offset = this.view.getUint32(start, true);
    const offset_end = this.view.getUint32(start + 4, true);
    return new Byte32(this.view.buffer.slice(offset, offset_end), { validate: false });
  }

  getProxyId() {
    const start = 8;
    const offset = this.view.getUint32(start, true);
    const offset_end = this.view.getUint32(start + 4, true);
    return new Byte32(this.view.buffer.slice(offset, offset_end), { validate: false });
  }

  getFrom() {
    const start = 12;
    const offset = this.view.getUint32(start, true);
    const offset_end = this.view.byteLength;
    return new Address(this.view.buffer.slice(offset, offset_end), { validate: false });
  }
}

export function SerializeBurnProxy(value) {
  const buffers = [];
  buffers.push(SerializeByte32(value.cluster_id));
  buffers.push(SerializeByte32(value.proxy_id));
  buffers.push(SerializeAddress(value.from));
  return serializeTable(buffers);
}

export class MintAgent {
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
    new Address(this.view.buffer.slice(offsets[2], offsets[3]), { validate: false }).validate();
  }

  getClusterId() {
    const start = 4;
    const offset = this.view.getUint32(start, true);
    const offset_end = this.view.getUint32(start + 4, true);
    return new Byte32(this.view.buffer.slice(offset, offset_end), { validate: false });
  }

  getProxyId() {
    const start = 8;
    const offset = this.view.getUint32(start, true);
    const offset_end = this.view.getUint32(start + 4, true);
    return new Byte32(this.view.buffer.slice(offset, offset_end), { validate: false });
  }

  getTo() {
    const start = 12;
    const offset = this.view.getUint32(start, true);
    const offset_end = this.view.byteLength;
    return new Address(this.view.buffer.slice(offset, offset_end), { validate: false });
  }
}

export function SerializeMintAgent(value) {
  const buffers = [];
  buffers.push(SerializeByte32(value.cluster_id));
  buffers.push(SerializeByte32(value.proxy_id));
  buffers.push(SerializeAddress(value.to));
  return serializeTable(buffers);
}

export class TransferAgent {
  constructor(reader, { validate = true } = {}) {
    this.view = new DataView(assertArrayBuffer(reader));
    if (validate) {
      this.validate();
    }
  }

  validate(compatible = false) {
    const offsets = verifyAndExtractOffsets(this.view, 0, true);
    new Byte32(this.view.buffer.slice(offsets[0], offsets[1]), { validate: false }).validate();
    new Address(this.view.buffer.slice(offsets[1], offsets[2]), { validate: false }).validate();
    new Address(this.view.buffer.slice(offsets[2], offsets[3]), { validate: false }).validate();
  }

  getClusterId() {
    const start = 4;
    const offset = this.view.getUint32(start, true);
    const offset_end = this.view.getUint32(start + 4, true);
    return new Byte32(this.view.buffer.slice(offset, offset_end), { validate: false });
  }

  getFrom() {
    const start = 8;
    const offset = this.view.getUint32(start, true);
    const offset_end = this.view.getUint32(start + 4, true);
    return new Address(this.view.buffer.slice(offset, offset_end), { validate: false });
  }

  getTo() {
    const start = 12;
    const offset = this.view.getUint32(start, true);
    const offset_end = this.view.byteLength;
    return new Address(this.view.buffer.slice(offset, offset_end), { validate: false });
  }
}

export function SerializeTransferAgent(value) {
  const buffers = [];
  buffers.push(SerializeByte32(value.cluster_id));
  buffers.push(SerializeAddress(value.from));
  buffers.push(SerializeAddress(value.to));
  return serializeTable(buffers);
}

export class BurnAgent {
  constructor(reader, { validate = true } = {}) {
    this.view = new DataView(assertArrayBuffer(reader));
    if (validate) {
      this.validate();
    }
  }

  validate(compatible = false) {
    const offsets = verifyAndExtractOffsets(this.view, 0, true);
    new Byte32(this.view.buffer.slice(offsets[0], offsets[1]), { validate: false }).validate();
    new Address(this.view.buffer.slice(offsets[1], offsets[2]), { validate: false }).validate();
  }

  getClusterId() {
    const start = 4;
    const offset = this.view.getUint32(start, true);
    const offset_end = this.view.getUint32(start + 4, true);
    return new Byte32(this.view.buffer.slice(offset, offset_end), { validate: false });
  }

  getFrom() {
    const start = 8;
    const offset = this.view.getUint32(start, true);
    const offset_end = this.view.byteLength;
    return new Address(this.view.buffer.slice(offset, offset_end), { validate: false });
  }
}

export function SerializeBurnAgent(value) {
  const buffers = [];
  buffers.push(SerializeByte32(value.cluster_id));
  buffers.push(SerializeAddress(value.from));
  return serializeTable(buffers);
}

export class SporeAction {
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
      new MintSpore(this.view.buffer.slice(4), { validate: false }).validate();
      break;
    case 1:
      new TransferSpore(this.view.buffer.slice(4), { validate: false }).validate();
      break;
    case 2:
      new BurnSpore(this.view.buffer.slice(4), { validate: false }).validate();
      break;
    case 3:
      new MintCluster(this.view.buffer.slice(4), { validate: false }).validate();
      break;
    case 4:
      new TransferCluster(this.view.buffer.slice(4), { validate: false }).validate();
      break;
    case 5:
      new MintProxy(this.view.buffer.slice(4), { validate: false }).validate();
      break;
    case 6:
      new TransferProxy(this.view.buffer.slice(4), { validate: false }).validate();
      break;
    case 7:
      new BurnProxy(this.view.buffer.slice(4), { validate: false }).validate();
      break;
    case 8:
      new MintAgent(this.view.buffer.slice(4), { validate: false }).validate();
      break;
    case 9:
      new TransferAgent(this.view.buffer.slice(4), { validate: false }).validate();
      break;
    case 10:
      new BurnAgent(this.view.buffer.slice(4), { validate: false }).validate();
      break;
    default:
      throw new Error(`Invalid type: ${t}`);
    }
  }

  unionType() {
    const t = this.view.getUint32(0, true);
    switch (t) {
    case 0:
      return "MintSpore";
    case 1:
      return "TransferSpore";
    case 2:
      return "BurnSpore";
    case 3:
      return "MintCluster";
    case 4:
      return "TransferCluster";
    case 5:
      return "MintProxy";
    case 6:
      return "TransferProxy";
    case 7:
      return "BurnProxy";
    case 8:
      return "MintAgent";
    case 9:
      return "TransferAgent";
    case 10:
      return "BurnAgent";
    default:
      throw new Error(`Invalid type: ${t}`);
    }
  }

  value() {
    const t = this.view.getUint32(0, true);
    switch (t) {
    case 0:
      return new MintSpore(this.view.buffer.slice(4), { validate: false });
    case 1:
      return new TransferSpore(this.view.buffer.slice(4), { validate: false });
    case 2:
      return new BurnSpore(this.view.buffer.slice(4), { validate: false });
    case 3:
      return new MintCluster(this.view.buffer.slice(4), { validate: false });
    case 4:
      return new TransferCluster(this.view.buffer.slice(4), { validate: false });
    case 5:
      return new MintProxy(this.view.buffer.slice(4), { validate: false });
    case 6:
      return new TransferProxy(this.view.buffer.slice(4), { validate: false });
    case 7:
      return new BurnProxy(this.view.buffer.slice(4), { validate: false });
    case 8:
      return new MintAgent(this.view.buffer.slice(4), { validate: false });
    case 9:
      return new TransferAgent(this.view.buffer.slice(4), { validate: false });
    case 10:
      return new BurnAgent(this.view.buffer.slice(4), { validate: false });
    default:
      throw new Error(`Invalid type: ${t}`);
    }
  }
}

export function SerializeSporeAction(value) {
  switch (value.type) {
  case "MintSpore":
    {
      const itemBuffer = SerializeMintSpore(value.value);
      const array = new Uint8Array(4 + itemBuffer.byteLength);
      const view = new DataView(array.buffer);
      view.setUint32(0, 0, true);
      array.set(new Uint8Array(itemBuffer), 4);
      return array.buffer;
    }
  case "TransferSpore":
    {
      const itemBuffer = SerializeTransferSpore(value.value);
      const array = new Uint8Array(4 + itemBuffer.byteLength);
      const view = new DataView(array.buffer);
      view.setUint32(0, 1, true);
      array.set(new Uint8Array(itemBuffer), 4);
      return array.buffer;
    }
  case "BurnSpore":
    {
      const itemBuffer = SerializeBurnSpore(value.value);
      const array = new Uint8Array(4 + itemBuffer.byteLength);
      const view = new DataView(array.buffer);
      view.setUint32(0, 2, true);
      array.set(new Uint8Array(itemBuffer), 4);
      return array.buffer;
    }
  case "MintCluster":
    {
      const itemBuffer = SerializeMintCluster(value.value);
      const array = new Uint8Array(4 + itemBuffer.byteLength);
      const view = new DataView(array.buffer);
      view.setUint32(0, 3, true);
      array.set(new Uint8Array(itemBuffer), 4);
      return array.buffer;
    }
  case "TransferCluster":
    {
      const itemBuffer = SerializeTransferCluster(value.value);
      const array = new Uint8Array(4 + itemBuffer.byteLength);
      const view = new DataView(array.buffer);
      view.setUint32(0, 4, true);
      array.set(new Uint8Array(itemBuffer), 4);
      return array.buffer;
    }
  case "MintProxy":
    {
      const itemBuffer = SerializeMintProxy(value.value);
      const array = new Uint8Array(4 + itemBuffer.byteLength);
      const view = new DataView(array.buffer);
      view.setUint32(0, 5, true);
      array.set(new Uint8Array(itemBuffer), 4);
      return array.buffer;
    }
  case "TransferProxy":
    {
      const itemBuffer = SerializeTransferProxy(value.value);
      const array = new Uint8Array(4 + itemBuffer.byteLength);
      const view = new DataView(array.buffer);
      view.setUint32(0, 6, true);
      array.set(new Uint8Array(itemBuffer), 4);
      return array.buffer;
    }
  case "BurnProxy":
    {
      const itemBuffer = SerializeBurnProxy(value.value);
      const array = new Uint8Array(4 + itemBuffer.byteLength);
      const view = new DataView(array.buffer);
      view.setUint32(0, 7, true);
      array.set(new Uint8Array(itemBuffer), 4);
      return array.buffer;
    }
  case "MintAgent":
    {
      const itemBuffer = SerializeMintAgent(value.value);
      const array = new Uint8Array(4 + itemBuffer.byteLength);
      const view = new DataView(array.buffer);
      view.setUint32(0, 8, true);
      array.set(new Uint8Array(itemBuffer), 4);
      return array.buffer;
    }
  case "TransferAgent":
    {
      const itemBuffer = SerializeTransferAgent(value.value);
      const array = new Uint8Array(4 + itemBuffer.byteLength);
      const view = new DataView(array.buffer);
      view.setUint32(0, 9, true);
      array.set(new Uint8Array(itemBuffer), 4);
      return array.buffer;
    }
  case "BurnAgent":
    {
      const itemBuffer = SerializeBurnAgent(value.value);
      const array = new Uint8Array(4 + itemBuffer.byteLength);
      const view = new DataView(array.buffer);
      view.setUint32(0, 10, true);
      array.set(new Uint8Array(itemBuffer), 4);
      return array.buffer;
    }
  default:
    throw new Error(`Invalid type: ${value.type}`);
  }
}

