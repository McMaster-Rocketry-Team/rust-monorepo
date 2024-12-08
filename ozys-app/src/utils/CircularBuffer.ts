// ChatGPT wrote this lol

export class CircularBuffer<T> {
  private buffer: (T | undefined)[]
  private capacity: number
  private head: number = 0 // Points to the start of the buffer
  private tail: number = 0 // Points to the end of the buffer
  private size: number = 0

  /**
   * Initializes the circular buffer with a given capacity.
   * @param capacity The maximum number of items the buffer can hold.
   */
  constructor(capacity: number) {
    if (capacity <= 0) {
      throw new Error('Capacity must be a positive integer')
    }
    this.capacity = capacity
    this.buffer = new Array<T | undefined>(capacity)
  }

  /**
   * Returns the current number of items in the buffer.
   * @returns The number of items in the buffer.
   */
  length(): number {
    return this.size
  }

  /**
   * Peeks at the item at the given index.
   * If the index is negative, it peeks from the end.
   * @param i The index to peek at.
   * @returns The item at the specified index.
   */
  peek(i: number): T | undefined {
    if (i >= this.size || i < -this.size) {
      return undefined
    }

    let index: number
    if (i >= 0) {
      index = (this.head + i) % this.capacity
    } else {
      index = (this.tail + i) % this.capacity
      if (index < 0) index += this.capacity
    }
    return this.buffer[index]
  }

  /**
   * Executes a provided function once for each item in the buffer.
   * @param callback The function to execute for each item.
   */
  forEach(callback: (value: T, index: number) => void): void {
    for (let i = 0; i < this.size; i++) {
      const index = (this.head + i) % this.capacity
      const item = this.buffer[index]
      callback(item!, i)
    }
  }

  /**
   * Adds an item to the start of the buffer.
   * If the buffer is full, it overwrites the oldest item.
   * @param value The item to add.
   */
  addStart(value: T): void {
    if (this.size === this.capacity) {
      // Overwrite the oldest item
      this.head = (this.head - 1 + this.capacity) % this.capacity
      this.buffer[this.head] = value
      this.tail = this.head // Tail moves to head since we overwrote the oldest
    } else {
      this.head = (this.head - 1 + this.capacity) % this.capacity
      this.buffer[this.head] = value
      this.size++
    }
  }

  /**
   * Adds an item to the end of the buffer.
   * If the buffer is full, it overwrites the oldest item.
   * @param value The item to add.
   */
  addLast(value: T): void {
    if (this.size === this.capacity) {
      // Overwrite the oldest item
      this.buffer[this.tail] = value
      this.head = (this.head + 1) % this.capacity
      this.tail = (this.tail + 1) % this.capacity
    } else {
      this.buffer[this.tail] = value
      this.tail = (this.tail + 1) % this.capacity
      this.size++
    }
  }

  /**
   * Converts the circular buffer to a regular array.
   * @returns An array containing all items in the buffer in order.
   */
  toArray(): T[] {
    const result: T[] = []
    for (let i = 0; i < this.size; i++) {
      const index = (this.head + i) % this.capacity
      const item = this.buffer[index]
      result.push(item!)
    }
    return result
  }

  /**
   * Retrieves the first N items from the buffer.
   * If there are fewer than N items, it returns all available items.
   * @param n The number of items to retrieve.
   * @returns An array containing the first N items.
   */
  firstN(n: number): T[] {
    if (n <= 0) return []
    const limit = Math.min(n, this.size)
    const result: T[] = []
    for (let i = 0; i < limit; i++) {
      const index = (this.head + i) % this.capacity
      const item = this.buffer[index]
      result.push(item!)
    }
    return result
  }

  /**
   * Removes and returns the first item from the buffer.
   * If the buffer is empty, it returns undefined.
   * @returns The removed item or undefined if the buffer is empty.
   */
  removeFirst(): T | undefined {
    if (this.size === 0) {
      return undefined
    }
    const item = this.buffer[this.head]
    this.buffer[this.head] = undefined
    this.head = (this.head + 1) % this.capacity
    this.size--
    return item
  }

  /**
   * Removes and returns the last item from the buffer.
   * If the buffer is empty, it returns undefined.
   * @returns The removed item or undefined if the buffer is empty.
   */
  removeLast(): T | undefined {
    if (this.size === 0) {
      return undefined
    }
    this.tail = (this.tail - 1 + this.capacity) % this.capacity
    const item = this.buffer[this.tail]
    this.buffer[this.tail] = undefined
    this.size--
    return item
  }

  /**
   * Retrieves the last N items from the buffer.
   * If there are fewer than N items, it returns all available items.
   * @param n The number of items to retrieve.
   * @returns An array containing the last N items.
   */
  lastN(n: number): T[] {
    if (n <= 0) return []
    const limit = Math.min(n, this.size)
    const result: T[] = []
    for (let i = this.size - limit; i < this.size; i++) {
      const index = (this.head + i) % this.capacity
      const item = this.buffer[index]
      result.push(item!)
    }
    return result
  }

  /**
   * Checks if the buffer is empty.
   * @returns True if the buffer is empty, false otherwise.
   */
  isEmpty(): boolean {
    return this.size === 0
  }

  /**
   * Clears the buffer, removing all items.
   */
  clear(): void {
    this.buffer = new Array<T | undefined>(this.capacity)
    this.head = 0
    this.tail = 0
    this.size = 0
  }
}
