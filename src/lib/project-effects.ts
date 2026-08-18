import {
  chooseProject,
  readProjectFile,
  recoverGoalState,
  refreshProject,
  writeProjectFile,
} from "./bridge";

export const projectSessionEffects = {
  choose: chooseProject,
  readFile: readProjectFile,
  refresh: refreshProject,
  writeFile: writeProjectFile,
  recover: recoverGoalState,
};
