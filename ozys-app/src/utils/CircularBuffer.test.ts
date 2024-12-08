// ChatGPT also wrote this

import { describe, expect, test, beforeEach } from 'vitest'
import { CircularBuffer } from './CircularBuffer'

describe('CircularBuffer', () => {
  let buffer: CircularBuffer<number>

  beforeEach(() => {
    // Initialize a new buffer before each test with a capacity of 5
    buffer = new CircularBuffer<number>(5)
  })

  describe('Constructor', () => {
    test('should initialize with correct capacity', () => {
      expect(buffer.length()).toBe(0)
    })

    test('should throw error for non-positive capacity', () => {
      expect(() => new CircularBuffer<number>(0)).toThrow(
        'Capacity must be a positive integer',
      )
      expect(() => new CircularBuffer<number>(-1)).toThrow(
        'Capacity must be a positive integer',
      )
    })
  })

  describe('addLast', () => {
    test('should add elements to the end', () => {
      buffer.addLast(1)
      buffer.addLast(2)
      buffer.addLast(3)
      expect(buffer.toArray()).toEqual([1, 2, 3])
      expect(buffer.length()).toBe(3)
    })

    test('should overwrite oldest elements when full', () => {
      buffer.addLast(1)
      buffer.addLast(2)
      buffer.addLast(3)
      buffer.addLast(4)
      buffer.addLast(5)
      expect(buffer.toArray()).toEqual([1, 2, 3, 4, 5])
      buffer.addLast(6) // Overwrites 1
      expect(buffer.toArray()).toEqual([2, 3, 4, 5, 6])
      expect(buffer.length()).toBe(5)
    })
  })

  describe('addStart', () => {
    test('should add elements to the start', () => {
      buffer.addStart(1)
      buffer.addStart(2)
      buffer.addStart(3)
      expect(buffer.toArray()).toEqual([3, 2, 1])
      expect(buffer.length()).toBe(3)
    })

    test('should overwrite oldest elements when full', () => {
      buffer.addLast(1)
      buffer.addLast(2)
      buffer.addLast(3)
      buffer.addLast(4)
      buffer.addLast(5)
      expect(buffer.toArray()).toEqual([1, 2, 3, 4, 5])
      buffer.addStart(0) // Overwrites 5
      expect(buffer.toArray()).toEqual([0, 1, 2, 3, 4])
      expect(buffer.length()).toBe(5)
    })
  })

  describe('peek', () => {
    beforeEach(() => {
      buffer.addLast(10)
      buffer.addLast(20)
      buffer.addLast(30)
      buffer.addLast(40)
      buffer.addLast(50)
    })

    test('should peek with positive indices', () => {
      expect(buffer.peek(0)).toBe(10)
      expect(buffer.peek(2)).toBe(30)
      expect(buffer.peek(4)).toBe(50)
      expect(buffer.peek(5)).toBeUndefined()
    })

    test('should peek with negative indices', () => {
      expect(buffer.peek(-1)).toBe(50)
      expect(buffer.peek(-3)).toBe(30)
      expect(buffer.peek(-5)).toBe(10)
      expect(buffer.peek(-6)).toBeUndefined()
    })

    test('should return undefined for out-of-bounds indices', () => {
      expect(buffer.peek(100)).toBeUndefined()
      expect(buffer.peek(-100)).toBeUndefined()
    })

    test('should handle peeks after overwriting', () => {
      buffer.addLast(60) // Overwrites 10
      expect(buffer.peek(0)).toBe(20)
      expect(buffer.peek(-1)).toBe(60)
    })
  })

  describe('toArray', () => {
    test('should return an empty array when buffer is empty', () => {
      expect(buffer.toArray()).toEqual([])
    })

    test('should return correct array after additions', () => {
      buffer.addLast(1)
      buffer.addStart(0)
      buffer.addLast(2)
      expect(buffer.toArray()).toEqual([0, 1, 2])
    })

    test('should maintain correct order after overwriting', () => {
      buffer.addLast(1)
      buffer.addLast(2)
      buffer.addLast(3)
      buffer.addLast(4)
      buffer.addLast(5)
      buffer.addLast(6) // Overwrites 1
      expect(buffer.toArray()).toEqual([2, 3, 4, 5, 6])
    })

    test('should handle mixed addStart and addLast operations', () => {
      buffer.addStart(1)
      buffer.addLast(2)
      buffer.addStart(0)
      buffer.addLast(3)
      buffer.addStart(-1)
      expect(buffer.toArray()).toEqual([-1, 0, 1, 2, 3])
    })
  })

  describe('firstN', () => {
    beforeEach(() => {
      buffer.addLast(100)
      buffer.addLast(200)
      buffer.addLast(300)
      buffer.addLast(400)
      buffer.addLast(500)
    })

    test('should return first N elements', () => {
      expect(buffer.firstN(3)).toEqual([100, 200, 300])
    })

    test('should return all elements if N exceeds size', () => {
      expect(buffer.firstN(10)).toEqual([100, 200, 300, 400, 500])
    })

    test('should return empty array if N is zero or negative', () => {
      expect(buffer.firstN(0)).toEqual([])
      expect(buffer.firstN(-1)).toEqual([])
    })

    test('should handle after overwriting', () => {
      buffer.addLast(600) // Overwrites 100
      expect(buffer.firstN(3)).toEqual([200, 300, 400])
    })
  })

  describe('lastN', () => {
    beforeEach(() => {
      buffer.addLast(10)
      buffer.addLast(20)
      buffer.addLast(30)
      buffer.addLast(40)
      buffer.addLast(50)
    })

    test('should return last N elements', () => {
      expect(buffer.lastN(2)).toEqual([40, 50])
    })

    test('should return all elements if N exceeds size', () => {
      expect(buffer.lastN(10)).toEqual([10, 20, 30, 40, 50])
    })

    test('should return empty array if N is zero or negative', () => {
      expect(buffer.lastN(0)).toEqual([])
      expect(buffer.lastN(-2)).toEqual([])
    })

    test('should handle after overwriting', () => {
      buffer.addLast(60) // Overwrites 10
      expect(buffer.lastN(3)).toEqual([40, 50, 60])
    })
  })

  describe('removeFirst', () => {
    test('should remove and return the first element', () => {
      buffer.addLast(1)
      buffer.addLast(2)
      buffer.addLast(3)
      expect(buffer.removeFirst()).toBe(1)
      expect(buffer.toArray()).toEqual([2, 3])
      expect(buffer.length()).toBe(2)
    })

    test('should return undefined when removing from empty buffer', () => {
      expect(buffer.removeFirst()).toBeUndefined()
    })

    test('should handle removing all elements', () => {
      buffer.addLast(10)
      buffer.addLast(20)
      buffer.removeFirst()
      buffer.removeFirst()
      expect(buffer.isEmpty()).toBe(true)
      expect(buffer.removeFirst()).toBeUndefined()
    })

    test('should handle after overwriting', () => {
      buffer.addLast(1)
      buffer.addLast(2)
      buffer.addLast(3)
      buffer.addLast(4)
      buffer.addLast(5)
      buffer.addLast(6) // Overwrites 1
      expect(buffer.removeFirst()).toBe(2)
      expect(buffer.toArray()).toEqual([3, 4, 5, 6])
      expect(buffer.length()).toBe(4)
    })
  })

  describe('removeLast', () => {
    test('should remove and return the last element', () => {
      buffer.addLast(1)
      buffer.addLast(2)
      buffer.addLast(3)
      expect(buffer.removeLast()).toBe(3)
      expect(buffer.toArray()).toEqual([1, 2])
      expect(buffer.length()).toBe(2)
    })

    test('should return undefined when removing from empty buffer', () => {
      expect(buffer.removeLast()).toBeUndefined()
    })

    test('should handle removing all elements', () => {
      buffer.addLast(10)
      buffer.addLast(20)
      buffer.removeLast()
      buffer.removeLast()
      expect(buffer.isEmpty()).toBe(true)
      expect(buffer.removeLast()).toBeUndefined()
    })

    test('should handle after overwriting', () => {
      buffer.addLast(1)
      buffer.addLast(2)
      buffer.addLast(3)
      buffer.addLast(4)
      buffer.addLast(5)
      buffer.addLast(6) // Overwrites 1
      expect(buffer.removeLast()).toBe(6)
      expect(buffer.toArray()).toEqual([2, 3, 4, 5])
      expect(buffer.length()).toBe(4)
    })
  })

  describe('isEmpty', () => {
    test('should return true for a new buffer', () => {
      expect(buffer.isEmpty()).toBe(true)
    })

    test('should return false after adding elements', () => {
      buffer.addLast(1)
      expect(buffer.isEmpty()).toBe(false)
    })

    test('should return true after removing all elements', () => {
      buffer.addLast(1)
      buffer.removeFirst()
      expect(buffer.isEmpty()).toBe(true)
    })
  })

  describe('clear', () => {
    test('should clear all elements from the buffer', () => {
      buffer.addLast(1)
      buffer.addLast(2)
      buffer.addLast(3)
      expect(buffer.length()).toBe(3)
      buffer.clear()
      expect(buffer.length()).toBe(0)
      expect(buffer.toArray()).toEqual([])
      expect(buffer.isEmpty()).toBe(true)
    })

    test('should handle clear on an already empty buffer', () => {
      buffer.clear()
      expect(buffer.isEmpty()).toBe(true)
      expect(buffer.toArray()).toEqual([])
    })
  })

  describe('Edge Cases', () => {
    test('should handle buffer with capacity 1', () => {
      const singleBuffer = new CircularBuffer<number>(1)
      expect(singleBuffer.isEmpty()).toBe(true)
      singleBuffer.addLast(100)
      expect(singleBuffer.toArray()).toEqual([100])
      expect(singleBuffer.length()).toBe(1)
      singleBuffer.addLast(200) // Overwrites 100
      expect(singleBuffer.toArray()).toEqual([200])
      expect(singleBuffer.length()).toBe(1)
      expect(singleBuffer.removeFirst()).toBe(200)
      expect(singleBuffer.isEmpty()).toBe(true)
    })

    test('should handle alternating addStart and addLast', () => {
      buffer.addLast(1)
      buffer.addStart(0)
      buffer.addLast(2)
      buffer.addStart(-1)
      buffer.addLast(3)
      expect(buffer.toArray()).toEqual([-1, 0, 1, 2, 3])
      buffer.addLast(4) // Overwrites -1
      expect(buffer.toArray()).toEqual([0, 1, 2, 3, 4])
      buffer.addStart(-2) // Overwrites 4
      expect(buffer.toArray()).toEqual([-2, 0, 1, 2, 3])
    })

    test('should handle multiple overwrites correctly', () => {
      for (let i = 1; i <= 10; i++) {
        buffer.addLast(i)
      }
      expect(buffer.toArray()).toEqual([6, 7, 8, 9, 10])
      for (let i = 11; i <= 15; i++) {
        buffer.addStart(i)
      }
      expect(buffer.toArray()).toEqual([15, 14, 13, 12, 11])
    })
  })
})
