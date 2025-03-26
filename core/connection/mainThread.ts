const MessageType = {
  domEvent: 0,
  custom: 1,
  attachCanvas: 2,
} as const;
type MessageTypeI = typeof MessageType;

const EventTypes = {
  pointermove: 0,
  pointerdown: 1,
  pointerup: 2,
  keydown: 3,
  keyup: 4,
  onWheel: 5,
} as const;
type EventTypesI = typeof EventTypes;

/**
 * @param worker The worker you're running the canvas on
 * @returns DisconnectWorker()
 */
const connectWorker = (worker: Worker) => {
  const eventPassers = {
    [EventTypes.pointermove]: (e: PointerEvent) => {
      worker.postMessage([
        MessageType.domEvent,
        EventTypes.pointermove,
        e.clientX,
        e.clientY,
      ]);
    },
    [EventTypes.pointerup]: (e: PointerEvent) => {
      worker.postMessage([
        MessageType.domEvent,
        EventTypes.pointerup,
        e.clientX,
        e.clientY,
      ]);
    },
    [EventTypes.pointerdown]: (e: PointerEvent) => {
      worker.postMessage([
        MessageType.domEvent,
        EventTypes.pointerdown,
        e.clientX,
        e.clientY,
      ]);
    },
    [EventTypes.keydown]: (e: KeyboardEvent) => {
      worker.postMessage([MessageType.domEvent, EventTypes.keydown, e.key]);
    },
    [EventTypes.keyup]: (e: KeyboardEvent) => {
      worker.postMessage([MessageType.domEvent, EventTypes.keyup, e.key]);
    },
    [EventTypes.onWheel]: () => {
      worker.postMessage([MessageType.domEvent, EventTypes.onWheel]);
    },
  } as const;

  document.addEventListener(
    "pointermove",
    eventPassers[EventTypes.pointermove],
  );
  document.addEventListener(
    "pointerdown",
    eventPassers[EventTypes.pointerdown],
  );
  document.addEventListener("pointerup", eventPassers[EventTypes.pointerup]);
  //
  document.addEventListener("keydown", eventPassers[EventTypes.keydown]);
  document.addEventListener("keyup", eventPassers[EventTypes.keyup]);

  const disconnectWorker = () => {
    document.removeEventListener(
      "pointermove",
      eventPassers[EventTypes.pointermove],
    );
    document.removeEventListener(
      "pointerdown",
      eventPassers[EventTypes.pointerdown],
    );
    document.removeEventListener(
      "pointerup",
      eventPassers[EventTypes.pointerup],
    );
    //
    document.removeEventListener("keydown", eventPassers[EventTypes.keydown]);
    document.removeEventListener("keyup", eventPassers[EventTypes.keyup]);
  };

  return disconnectWorker;
};

const attachCanvas = (worker: Worker, canvas: HTMLCanvasElement | string) => {
  if (typeof canvas === "string")
    canvas = document.getElementById("rendering-canvas") as HTMLCanvasElement;
  if (!canvas) {
    throw new Error("Fatal: Canvas Not Found!");
  }

  canvas.width = canvas.clientWidth;
  canvas.height = canvas.clientHeight;

  const canvasWorker = canvas.transferControlToOffscreen();
  worker.postMessage([MessageType.attachCanvas, canvasWorker], [canvasWorker]);
};

export type { EventTypesI, MessageTypeI };
export { EventTypes, connectWorker, attachCanvas, MessageType };
