import { useOrganizations } from "../../hooks/useOrganizations.ts";
import { useStreams } from "../../hooks/useStreams.ts";
import { useProjects } from "../../hooks/useProjects.ts";
import { useCredentials } from "../../hooks/useCredentials.ts";
import { useRemails } from "../../hooks/useRemails.ts";
import { useForm } from "@mantine/form";
import { useEffect } from "react";
import { Loader } from "../../Loader.tsx";
import { SmtpCredential } from "../../types.ts";
import { modals } from "@mantine/modals";
import { Button, Container, Group, Stack, Text, Textarea, TextInput, Tooltip } from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { IconKey, IconTrash, IconX } from "@tabler/icons-react";
import EntityHeader from "../EntityHeader.tsx";

interface FormValues {
  description: string;
}

export default function CredentialDetails() {
  const { currentOrganization } = useOrganizations();
  const { currentStream } = useStreams();
  const { currentProject } = useProjects();
  const { currentCredential } = useCredentials();
  const { dispatch, navigate } = useRemails();

  const form = useForm<FormValues>({
    initialValues: {
      description: "",
    },
  });

  useEffect(() => {
    form.setValues({
      description: currentCredential?.description || "",
    });
    form.resetDirty();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currentCredential]);

  if (!currentStream || !currentOrganization || !currentProject || !currentCredential) {
    return <Loader />;
  }

  const confirmDeleteCredential = (credential: SmtpCredential) => {
    modals.openConfirmModal({
      title: "Please confirm your action",
      children: (
        <Text>
          Are you sure you want to delete the SMTP credential with the username <strong>{credential.username}</strong>?
          You won't be able to sent messages with this credential anymore. This action cannot be undone.
        </Text>
      ),
      labels: { confirm: "Confirm", cancel: "Cancel" },
      onCancel: () => {},
      onConfirm: () => deleteCredential(credential),
    });
  };

  const deleteCredential = async (credential: SmtpCredential) => {
    const res = await fetch(
      `/api/organizations/${currentOrganization.id}/projects/${currentProject.id}/streams/${currentStream.id}/smtp_credentials/${credential.id}`,
      {
        method: "DELETE",
      }
    );
    if (res.status === 200) {
      notifications.show({
        title: "Credential deleted",
        message: `Credential with username ${credential.username} deleted`,
        color: "green",
      });
      navigate("projects.project.streams.stream.credentials", {});
      dispatch({ type: "remove_credential", credentialId: credential.id });
    } else {
      notifications.show({
        title: "Error",
        message: `Credential with username ${credential.username} could not be deleted`,
        color: "red",
        autoClose: 20000,
        icon: <IconX size={20} />,
      });
      console.error(res);
    }
  };

  const save = async (values: FormValues) => {
    const res = await fetch(
      `/api/organizations/${currentOrganization.id}/projects/${currentProject.id}/streams/${currentStream.id}/smtp_credentials/${currentCredential.id}`,
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
        message: `SMTP credential could not be updated`,
        color: "red",
        autoClose: 20000,
        icon: <IconX size={20} />,
      });
      console.error(res);
      return;
    }
    const credential = await res.json();

    notifications.show({
      title: "SMTP credential updated",
      message: "",
      color: "green",
    });
    dispatch({ type: "remove_credential", credentialId: credential.id });
    dispatch({ type: "add_credential", credential });
  };

  return (
    <>
      <Container size="sm" ml="0" pl="0">
        <EntityHeader name={currentCredential?.username || ""} entityType="SMTP Credential" Icon={IconKey} />
        <form onSubmit={form.onSubmit(save)}>
          <Stack>
            <TextInput variant="filled" label="Username" value={currentCredential?.username || ""} readOnly />
            <Textarea
              label="Description"
              autosize
              maxRows={10}
              key={form.key("name")}
              value={form.values.description}
              onChange={(event) => form.setFieldValue("description", event.currentTarget.value)}
            />
            <Tooltip label="The password cannot be shown or changed. Please create a new credential if needed and possibly delete this one.">
              <TextInput label="Password" value="••••••••" readOnly variant="filled" />
            </Tooltip>
            <Group>
              <Tooltip label="Delete SMTP credential">
                <Button
                  leftSection={<IconTrash />}
                  variant="outline"
                  onClick={() => confirmDeleteCredential(currentCredential)}
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
      </Container>
    </>
  );
}
