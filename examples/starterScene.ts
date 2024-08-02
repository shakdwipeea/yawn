import { handleConnection } from "../core/connection/workerThread";

self.onmessage = handleConnection;
