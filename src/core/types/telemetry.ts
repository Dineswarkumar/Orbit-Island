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
  childProcesses: ProcessItem[];
  unmeasuredDescendantCount: number;
  totalWorkingSetMb: number;
  totalPrivateMb: number;
  totalPrivateWorkingSetMb: number | null;
  aspirationMb: number;
  targetMb: number;
  maximumMb: number;
  meetsAspiration: boolean;
  meetsTarget: boolean;
  meetsMaximum: boolean;
  measurementMethod: string;
}
