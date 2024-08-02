import { MessageType } from "./mainThread";

const handleConnection = (msg: MessageEvent<any>) => {
  const { data } = msg;

  if (!(data instanceof Array)) return;
  if (!data.length) return;

  switch (data[0]) {
    case MessageType.attachCanvas:
      const canvas = data[1];
      const ctxWorker = canvas.getContext("2d");

      ctxWorker.clearRect(0, 0, canvas.width, canvas.height);
      ctxWorker.font = "24px Verdana";
      ctxWorker.textAlign = "center";
      ctxWorker.fillText("Hello World", canvas.width / 2, canvas.height / 2);

      break;
  }

  console.log(...data);
}

export { handleConnection }; 
