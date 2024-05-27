import {
  ForwardedRef,
  forwardRef,
  useCallback,
  useImperativeHandle,
  useReducer,
  useRef,
  useState,
} from "react";
import { FixedSizeList } from "react-window";
import { useMeasure } from "react-use";
import { FLASH_SIZE } from "./Flash";
import {
  WsRequest,
  getRequestRange,
  useHandleWebsocket,
} from "./useHandleWebsocket";

const LIST_ITEM_HEIGHT = 22;
function App() {
  const [addressText, setAddressText] = useState("");
  const prevFlashRef = useRef<FlashContentRef>(null);
  const currentFlashRef = useRef<FlashContentRef>(null);

  const forceUpdate = useCallback(() => {
    prevFlashRef.current?.forceUpdate();
    currentFlashRef.current?.forceUpdate();
  }, []);

  const { getCurrentByte, getPrevByte, pendingRequest, resume, connected } =
    useHandleWebsocket(forceUpdate);

  const jumpToAddress = (address: number) => {
    const offset = (address / 16) * LIST_ITEM_HEIGHT;
    prevFlashRef.current?.scrollTo(offset);
    currentFlashRef.current?.scrollTo(offset);
  };

  return (
    <div
      style={{
        display: "flex",
        alignItems: "flex-start",
        minHeight: "100vh",
        gap: 16,
      }}
    >
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          alignItems: "flex-start",
          gap: 8,
          marginLeft: 16,
        }}
      >
        <h1>Jump To</h1>
        <button onClick={() => jumpToAddress(0)}>Allocation Table #1</button>
        <button onClick={() => jumpToAddress(0x8000)}>
          Allocation Table #2
        </button>
        <button onClick={() => jumpToAddress(0x10000)}>
          Allocation Table #3
        </button>
        <button onClick={() => jumpToAddress(0x18000)}>
          Allocation Table #4
        </button>
        <form
          onSubmit={(e) => {
            e.preventDefault();
            const address = parseInt(addressText, 16);
            jumpToAddress(address);
            setAddressText("");
          }}
        >
          <input
            type="text"
            value={addressText}
            onChange={(e) => setAddressText(e.target.value)}
          />
        </form>
        {pendingRequest && <button onClick={resume}>Resume</button>}
        <p>{connected ? "Connected" : "Disconnected"}</p>
      </div>
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "auto auto",
          alignSelf: "stretch",
          gridTemplateRows: "max-content 1fr",
          columnGap: 16,
        }}
      >
        <h1>Previous Flash</h1>
        <h1>Current Flash</h1>
        <FlashContent
          ref={prevFlashRef}
          onScroll={(offset) => {
            currentFlashRef.current?.scrollTo(offset);
          }}
          getByte={getPrevByte}
          request={pendingRequest}
        />
        <FlashContent
          ref={currentFlashRef}
          onScroll={(offset) => {
            prevFlashRef.current?.scrollTo(offset);
          }}
          getByte={getCurrentByte}
          request={pendingRequest}
        />
      </div>
    </div>
  );
}

type FlashContentRef = {
  scrollTo: (offset: number) => void;
  forceUpdate: () => void;
};

const FlashContent = forwardRef(function FlashContent(
  props: {
    onScroll?: (offset: number) => void;
    getByte: (address: number) => number;
    request: WsRequest | null;
  },
  ref: ForwardedRef<FlashContentRef>
) {
  const [containerRef, { height }] = useMeasure<HTMLDivElement>();
  const addressListRef = useRef<FixedSizeList>(null);
  const dataListRef = useRef<FixedSizeList>(null);
  const [, forceUpdate] = useReducer((x) => x + 1, 0);
  useImperativeHandle(
    ref,
    () => {
      return {
        scrollTo: (offset: number) => {
          addressListRef.current?.scrollTo(offset);
          dataListRef.current?.scrollTo(offset);
        },
        forceUpdate,
      };
    },
    []
  );
  const range = props.request ? getRequestRange(props.request) : [0, 0];
  let highlightColor = "";
  if (
    props.request?.type === "eraseSector4Kib" ||
    props.request?.type === "eraseBlock32Kib" ||
    props.request?.type === "eraseBlock64Kib"
  ) {
    highlightColor = "#cbd5e1";
  } else if (props.request?.type === "read") {
    highlightColor = "#38bdf8";
  } else if (props.request?.type === "write256b") {
    highlightColor = "#fdba74";
  }

  return (
    <div
      ref={containerRef}
      style={{
        display: "flex",
      }}
    >
      <FixedSizeList
        ref={addressListRef}
        itemSize={LIST_ITEM_HEIGHT}
        itemCount={FLASH_SIZE / 16}
        height={height}
        width={77 + 8}
        className="flash-list"
        onScroll={(e) => {
          dataListRef.current?.scrollTo(e.scrollOffset);
          props.onScroll?.(e.scrollOffset);
        }}
      >
        {({ index, style }) => {
          const address = index * 16;
          const addressText = address
            .toString(16)
            .padStart(8, "0")
            .toUpperCase();
          return (
            <div
              style={{
                ...style,
              }}
            >
              <span>{addressText}</span>
            </div>
          );
        }}
      </FixedSizeList>
      <FixedSizeList
        ref={dataListRef}
        itemSize={LIST_ITEM_HEIGHT}
        itemCount={FLASH_SIZE / 16}
        height={height}
        width={452}
        className="flash-list"
        onScroll={(e) => {
          addressListRef.current?.scrollTo(e.scrollOffset);
          props.onScroll?.(e.scrollOffset);
        }}
      >
        {({ index, style }) => {
          const address = index * 16;
          const byteComponents = [];
          for (let i = 0; i < 16; i++) {
            const byte = props.getByte(address + i);
            const isInRange = address + i >= range[0] && address + i < range[1];
            byteComponents.push(
              <span
                key={i}
                style={{
                  paddingLeft: 4,
                  paddingRight: 4,
                  backgroundColor: isInRange ? highlightColor : undefined,
                }}
              >
                {byte.toString(16).padStart(2, "0").toUpperCase()}
              </span>
            );
          }
          return (
            <div
              style={{
                ...style,
              }}
            >
              {byteComponents}
            </div>
          );
        }}
      </FixedSizeList>
    </div>
  );
});

export default App;
