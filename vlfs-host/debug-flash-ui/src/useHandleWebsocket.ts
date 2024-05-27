import { useCallback, useEffect, useRef, useState } from "react";
import useWebSocket, { ReadyState } from "react-use-websocket";
import { z } from "zod";
import { Flash } from "./Flash";

const wsRequestSchema = z.discriminatedUnion("type", [
  z.object({
    type: z.literal("eraseSector4Kib"),
    address: z.number(),
  }),
  z.object({
    type: z.literal("eraseBlock32Kib"),
    address: z.number(),
  }),
  z.object({
    type: z.literal("eraseBlock64Kib"),
    address: z.number(),
  }),
  z.object({
    type: z.literal("read"),
    address: z.number(),
    length: z.number(),
  }),
  z.object({
    type: z.literal("write256b"),
    address: z.number(),
    data: z.array(z.number()),
  }),
]);

type WsRequest = z.infer<typeof wsRequestSchema>;

export function getRequestRange(request: WsRequest) {
  switch (request.type) {
    case "eraseSector4Kib":
      return [request.address, request.address + 4096];
    case "eraseBlock32Kib":
      return [request.address, request.address + 32768];
    case "eraseBlock64Kib":
      return [request.address, request.address + 65536];
    case "read":
      return [request.address, request.address + request.length];
    case "write256b":
      return [request.address, request.address + 256];
  }
}

export function useHandleWebsocket(forceUpdate: () => void) {
  const flashRef = useRef<Flash>();

  useEffect(() => {
    flashRef.current = new Flash();
  }, []);
  const { sendMessage, lastMessage, readyState } = useWebSocket("http://localhost:19000");
  const [pendingRequest, setPendingRequest] = useState<WsRequest | null>(null);

  const handleWsRequest = useCallback(
    (request: WsRequest) => {
      setPendingRequest(request);
      forceUpdate();
    },
    [forceUpdate]
  );

  useEffect(() => {
    if (lastMessage) {
      try {
        const request = wsRequestSchema.parse(JSON.parse(lastMessage.data));
        handleWsRequest(request);
      } catch (e) {
        console.warn("Failed to parse websocket request", e);
      }
    }
  }, [lastMessage, handleWsRequest]);

  const getPrevByte = useCallback((address: number) => {
    return flashRef.current?.getByte(address) ?? 0;
  }, []);

  const getCurrentByte = useCallback(
    (address: number) => {
      if (pendingRequest) {
        const [start, end] = getRequestRange(pendingRequest);
        if (address >= start && address < end) {
          if (
            pendingRequest.type === "eraseSector4Kib" ||
            pendingRequest.type === "eraseBlock32Kib" ||
            pendingRequest.type === "eraseBlock64Kib"
          ) {
            return 0xff;
          } else if (pendingRequest.type === "write256b") {
            return pendingRequest.data[address - pendingRequest.address];
          }
        }
      }
      return getPrevByte(address);
    },
    [pendingRequest, getPrevByte]
  );

  const resume = useCallback(() => {
    if (pendingRequest) {
      if (
        pendingRequest.type === "eraseSector4Kib" ||
        pendingRequest.type === "eraseBlock32Kib" ||
        pendingRequest.type === "eraseBlock64Kib"
      ) {
        flashRef.current![pendingRequest.type](pendingRequest.address);
        sendMessage("ok");
      } else if (pendingRequest.type === "read") {
        const data = flashRef.current!.read(
          pendingRequest.address,
          pendingRequest.length
        );
        sendMessage("ok" + JSON.stringify(data));
      } else if (pendingRequest.type === "write256b") {
        flashRef.current!.write256b(
          pendingRequest.address,
          new Uint8Array(pendingRequest.data)
        );
        sendMessage("ok");
      }
      setPendingRequest(null);
      forceUpdate();
    }
  }, [pendingRequest, sendMessage, forceUpdate]);

  return {
    getPrevByte,
    getCurrentByte,
    pendingRequest,
    resume,
    connected:readyState===ReadyState.OPEN
  };
}
