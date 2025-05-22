import { Loader } from "../../Loader.tsx";
import { useStreams } from "../../hooks/useStreams.ts";
import { Button, Grid, Group, Stack, Text, TextInput, Tooltip } from "@mantine/core";
import { useForm } from "@mantine/form";
import { IconTrash, IconX } from "@tabler/icons-react";
import { useEffect, useState } from "react";
import { useMessages } from "../../hooks/useMessages.ts";
import { Project } from "../../types.ts";
import { modals } from "@mantine/modals";
import { notifications } from "@mantine/notifications";
import { useOrganizations } from "../../hooks/useOrganizations.ts";
import { useRemails } from "../../hooks/useRemails.ts";
import { useProjects } from "../../hooks/useProjects.ts";
import { CredentialsOverview } from "../smtpCredentials/CredentialsOverview.tsx";
import { MessageLog } from "../messages/MessageLog.tsx";

interface FormValues {
  name: string;
}

export default function StreamDetails() {
  const [canDelete, setCanDelete] = useState<boolean>(false);
  const { currentOrganization } = useOrganizations();
  const { messages } = useMessages();
  const { currentStream } = useStreams();
  const { currentProject } = useProjects();
  const { dispatch, navigate } = useRemails();

  useEffect(() => {
    if (messages && messages.length === 0) {
      setCanDelete(true);
    } else {
      setCanDelete(false);
    }
  }, [messages]);

  const form = useForm<FormValues>({
    initialValues: {
      name: "",
    },
  });

  useEffect(() => {
    form.setValues({ name: currentStream?.name || "" });
    form.resetDirty();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currentStream]);

  if (!currentStream || !currentOrganization || !currentProject) {
    return <Loader />;
  }

  const confirmDeleteStream = (project: Project) => {
    modals.openConfirmModal({
      title: "Please confirm your action",
      children: (
        <Text>
          Are you sure you want to delete Stream <strong>{project.name}</strong>? This action cannot be undone
        </Text>
      ),
      labels: { confirm: "Confirm", cancel: "Cancel" },
      onCancel: () => {},
      onConfirm: () => deleteStream(project),
    });
  };

  const deleteStream = async (stream: Project) => {
    const res = await fetch(
      `/api/organizations/${currentOrganization.id}/projects/${currentProject.id}/streams/${stream.id}`,
      {
        method: "DELETE",
      }
    );
    if (res.status === 200) {
      notifications.show({
        title: "Stream deleted",
        message: `Stream ${stream.name} deleted`,
        color: "green",
      });
      dispatch({ type: "remove_stream", streamId: stream.id });
      navigate("projects.project.streams");
    } else {
      notifications.show({
        title: "Error",
        message: `Stream ${stream.name} could not be deleted`,
        color: "red",
        autoClose: 20000,
        icon: <IconX size={20} />,
      });
      console.error(res);
    }
  };

  const save = async (values: FormValues) => {
    const res = await fetch(
      `/api/organizations/${currentOrganization.id}/projects/${currentProject.id}/streams/${currentStream.id}`,
      {
        method: "PUT",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify(values),
      }
    );
    if (res.status !== 200) {
      notifications.show({
        title: "Error",
        message: `Streams could not be updated`,
        color: "red",
        autoClose: 20000,
        icon: <IconX size={20} />,
      });
      console.error(res);
      return;
    }
    const stream = await res.json();

    notifications.show({
      title: "Stream updated",
      message: "",
      color: "green",
    });
    dispatch({ type: "remove_stream", streamId: currentStream.id });
    dispatch({ type: "add_stream", stream });
  };

  return (
    <Grid gutter="xl">
      <Grid.Col span={{ base: 12, md: 6, lg: 3 }}>
        <h2>Stream Details</h2>
        <form onSubmit={form.onSubmit(save)}>
          <Stack>
            <TextInput
              label="Name"
              key={form.key("name")}
              value={form.values.name}
              onChange={(event) => form.setFieldValue("name", event.currentTarget.value)}
            />
            <Group>
              <Tooltip
                label={canDelete ? "Delete Stream" : "Cannot delete Stream, there are Messages in it"}
                events={{ focus: false, hover: true, touch: true }}
              >
                <Button
                  leftSection={<IconTrash />}
                  color="red"
                  disabled={!canDelete}
                  onClick={() => confirmDeleteStream(currentStream)}
                >
                  Delete
                </Button>
              </Tooltip>
              <Button type="submit" disabled={!form.isDirty()} loading={form.submitting}>
                Save
              </Button>
            </Group>
          </Stack>
        </form>
      </Grid.Col>
      <Grid.Col span={{ base: 12, md: 6, lg: 9 }}>
        <h2>Credentials</h2>
        <CredentialsOverview />
      </Grid.Col>
      <Grid.Col span={12}>
        <h2>Messages</h2>
        <MessageLog />
      </Grid.Col>
    </Grid>
  );
}
