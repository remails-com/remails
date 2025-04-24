import {Loader} from "../../Loader.tsx";
import {Stream} from "../../types.ts";

export interface StreamDetailsProps {
  currentStream: Stream;
}

export function StreamDetails({currentStream}: StreamDetailsProps) {
  if (!currentStream) {
    return <Loader/>;
  }

  return (
    <>
      ID: {currentStream.id}
      <br/>
      Name: {currentStream.name}
      <br/>
    </>

  )
}