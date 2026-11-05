export interface ProcessItem {
  pid: number;
  name: string;
  workingSetBytes: number;
  workingSetMb: number;
  privateBytes: number;
  privateMb: number;
  privateWorkingSetMb: number | null;
}

export interface MemoryReport {
  mainProcess: ProcessItem;
  webviewProcesses: ProcessItem[];
  unmeasuredWebviewCount: number;
  combinedPrivateWorkingSetMb: number | null;
  measuredAtUnixMs: number;
  measurementMethod: string;
}
