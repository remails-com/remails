import {Loader} from "@mantine/core";
import {Login} from "./Login";
import {Pages} from "./Pages";
import {RouterContext, useInitRouter} from "./hooks/useRouter";
import {useLoadUser, UserContext} from "./hooks/useUser";
import {ProjectContext, useLoadProjects} from "./hooks/useProjects.ts";
import {StreamContext, useLoadStreams} from "./hooks/useStreams.ts";
import {RemailsContext, useLoadRemails} from "./hooks/useRemails.ts";

export default function App() {
  const {user, loading, setUser} = useLoadUser();
  const {state, dispatch} = useLoadRemails();
  const {projects, setCurrentProject, currentProject, loading: loadingProject} = useLoadProjects(state.currentOrganization);
  const {
    currentStream,
    setCurrentStream,
    streams,
    loading: loadingStream
  } = useLoadStreams(state.currentOrganization, currentProject);
  const {
    params,
    route,
    navigate,
    fullPath
  } = useInitRouter();

  if (loading) {
    return <Loader color="gray" size="xl" type="dots"/>;
  }

  if (!user) {
    return <Login setUser={setUser}/>;
  }

  return (
    <RouterContext.Provider value={{params, route, navigate, fullPath}}>
      <UserContext.Provider value={user}>
        <RemailsContext.Provider value={{state, dispatch}}>
          <ProjectContext.Provider value={{projects, setCurrentProject, currentProject, loading: loadingProject}}>
            <StreamContext.Provider value={{streams, setCurrentStream, currentStream, loading: loadingStream}}>
              <Pages/>
            </StreamContext.Provider>
          </ProjectContext.Provider>
        </RemailsContext.Provider>
      </UserContext.Provider>
    </RouterContext.Provider>
  );
}
