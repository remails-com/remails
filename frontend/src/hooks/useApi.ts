import { Dispatch, useEffect } from "react";
import { Action, State, WhoamiResponse } from "../types";
import { Navigate } from "../router";

export function useApi(user: WhoamiResponse | null, state: State, navigate: Navigate, dispatch: Dispatch<Action>) {
  useEffect(() => {
    if (user) {
      fetch("/api/organizations")
        .then((res) => res.json())
        .then((data) => {
          if (Array.isArray(data)) {
            // TODO store this somehow, e.g., as cookie or in local storage
            dispatch({ type: "set_organizations", organizations: data });
            if (!state.routerState.params.org_id && data.length > 0) {
              navigate("projects", { org_id: data[0].id });
            }
          }
        });
    } else {
      dispatch({ type: "set_organizations", organizations: null });
    }
  }, [dispatch, user, navigate, state.routerState.params.org_id]);

  // fetch projects when current organization changes
  useEffect(() => {
    const id = state.routerState.params.org_id;

    if (id) {
      fetch(`/api/organizations/${id}/projects`)
        .then((res) => res.json())
        .then((data) => {
          if (Array.isArray(data)) {
            dispatch({ type: "set_projects", projects: data });
          }
        });
    } else {
      dispatch({ type: "set_projects", projects: null });
    }
  }, [dispatch, user, state.routerState.params.org_id]);

  // fetch streams when current project changes
  useEffect(() => {
    const org_id = state.routerState.params.org_id;
    const proj_id = state.routerState.params.proj_id;

    if (org_id && proj_id) {
      fetch(`/api/organizations/${org_id}/projects/${proj_id}/streams`)
        .then((res) => res.json())
        .then((data) => {
          if (Array.isArray(data)) {
            dispatch({ type: "set_streams", streams: data });
          }
        });
    } else {
      dispatch({ type: "set_streams", streams: null });
    }
  }, [dispatch, user, state.routerState.params.org_id, state.routerState.params.proj_id]);

  useEffect(() => {
    const org_id = state.routerState.params.org_id;
    const proj_id = state.routerState.params.proj_id;
    const stream_id = state.routerState.params.stream_id;

    if (org_id && proj_id && stream_id) {
      fetch(`/api/organizations/${org_id}/projects/${proj_id}/streams/${stream_id}/messages`)
        .then((res) => res.json())
        .then((data) => {
          if (Array.isArray(data)) {
            dispatch({ type: "set_messages", messages: data });
          }
        });
    } else {
      dispatch({ type: "set_messages", messages: null });
    }
  }, [
    dispatch,
    user,
    state.routerState.params.org_id,
    state.routerState.params.proj_id,
    state.routerState.params.stream_id,
  ]);

  useEffect(() => {
    const org_id = state.routerState.params.org_id;
    const proj_id = state.routerState.params.proj_id;

    let url: string;
    if (org_id && proj_id) {
      url = `/api/organizations/${org_id}/projects/${proj_id}/domains`;
    } else if (org_id) {
      url = `/api/organizations/${org_id}/domains`;
    } else {
      dispatch({ type: "set_domains", domains: null });
      return;
    }

    fetch(url)
      .then((res) => res.json())
      .then((data) => {
        if (Array.isArray(data)) {
          dispatch({ type: "set_domains", domains: data });
        }
      });
  }, [dispatch, user, state.routerState.params.org_id, state.routerState.params.proj_id]);

  useEffect(() => {
    const org_id = state.routerState.params.org_id;
    const proj_id = state.routerState.params.proj_id;
    const stream_id = state.routerState.params.stream_id;

    if (org_id && proj_id && stream_id) {
      fetch(`/api/organizations/${org_id}/projects/${proj_id}/streams/${stream_id}/smtp_credentials`)
        .then((res) => res.json())
        .then((data) => {
          if (Array.isArray(data)) {
            dispatch({ type: "set_credentials", credentials: data });
          }
        });
    } else {
      dispatch({ type: "set_credentials", credentials: null });
      return;
    }
  }, [
    dispatch,
    user,
    state.routerState.params.org_id,
    state.routerState.params.proj_id,
    state.routerState.params.stream_id,
  ]);

  useEffect(() => {
    fetch("/api/config")
      .then((res) => res.json())
      .then((data) => dispatch({ type: "set_config", config: data }));
  }, [dispatch]);
}
