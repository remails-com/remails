import {Loader} from "../../Loader.tsx";
import {useStreams} from "../../hooks/useStreams.ts";

export function Stream() {
  const {currentStream} = useStreams()

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