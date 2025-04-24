import {Loader} from "../../Loader.tsx";
import {useStreams} from "../../hooks/useStreams.ts";
import {MessageLog} from "../MessageLog.tsx";


export function StreamDetails() {
  const {currentStream} = useStreams();

  if (!currentStream) {
    return <Loader/>;
  }

  return (
    <>
      ID: {currentStream.id}
      <br/>
      Name: {currentStream.name}
      <br/>

      <h2>Messages</h2>
      <MessageLog/>
    </>

  )
}