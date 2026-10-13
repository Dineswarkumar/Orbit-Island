export interface ProcessItem {
  pid: number;
  name: string;
  workingSetBytes: number;
  workingSetMb: number;
  privateBytes: number;
  privateMb: number;
}

export interface MemoryReport {
  mainProcess: ProcessItem;
  childProcesses: ProcessItem[];
  totalWorkingSetMb: number;
  totalPrivateMb: number;
  aspirationMb: number;
  targetMb: number;
  meetsAspiration: boolean;
  meetsTarget: boolean;
  measurementMethod: string;
}
