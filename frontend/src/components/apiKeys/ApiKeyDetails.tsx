import { useOrganizations } from "../../hooks/useOrganizations.ts";
import { useApiKeys } from "../../hooks/useApiKeys.ts";
import { useRemails } from "../../hooks/useRemails.ts";
import { useForm } from "@mantine/form";
import { useEffect } from "react";
import { Loader } from "../../Loader.tsx";
import { ApiKey, KeyRole } from "../../types.ts";
import { modals } from "@mantine/modals";
import { Container, Group, Select, Stack, Text, Textarea, TextInput, Tooltip } from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { IconKey, IconTrash } from "@tabler/icons-react";
import Header from "../Header.tsx";
import { errorNotification } from "../../notify.tsx";
import { MaintainerButton } from "../RoleButtons.tsx";
import { isValidKeyRole, roleSelectData } from "./NewApiKey.tsx";

interface FormValues {
  description: string;
  role: KeyRole;
}

export default function ApiKeyDetails() {
  const { currentOrganization } = useOrganizations();
  const { currentApiKey } = useApiKeys();
  const { dispatch, navigate } = useRemails();

  const form = useForm<FormValues>({
    initialValues: {
      description: "",
      role: "maintainer",
    },
  });

  useEffect(() => {
    form.setValues({
      description: currentApiKey?.description || "",
      role: currentApiKey?.role || "maintainer",
    });
    form.resetDirty();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currentApiKey]);

  if (!currentOrganization || !currentApiKey) {
    return <Loader />;
  }

  const confirmDeleteApiKey = (apiKey: ApiKey) => {
    modals.openConfirmModal({
      title: "Please confirm your action",
      children: (
        <Text>
          Are you sure you want to delete the API key with the description <strong>{apiKey.description}</strong>? You
          won't be able to use this key to access the Remails API anymore. This action cannot be undone.
        </Text>
      ),
      labels: { confirm: "Confirm", cancel: "Cancel" },
      onCancel: () => {},
      onConfirm: () => deleteApiKey(apiKey),
    });
  };

  const deleteApiKey = async (apiKey: ApiKey) => {
    const res = await fetch(`/api/organizations/${currentOrganization.id}/api_keys/${apiKey.id}`, {
      method: "DELETE",
    });
    if (res.status === 200) {
      notifications.show({
        title: "API key deleted",
        message: `API key with description ${apiKey.description} deleted`,
        color: "green",
      });
      navigate("settings.API keys", {});
      dispatch({ type: "remove_api_key", apiKeyId: apiKey.id });
    } else {
      errorNotification(`API key with description ${apiKey.description} could not be deleted`);
      console.error(res);
    }
  };

  const save = async (values: FormValues) => {
    const res = await fetch(`/api/organizations/${currentOrganization.id}/api_keys/${currentApiKey.id}`, {
      method: "PUT",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(values),
    });
    if (res.status !== 200) {
      errorNotification("API key could not be updated");
      console.error(res);
      return;
    }
    const apiKey = await res.json();

    notifications.show({
      title: "API key updated",
      message: "",
      color: "green",
    });
    dispatch({ type: "remove_api_key", apiKeyId: apiKey.id });
    dispatch({ type: "add_api_key", apiKey });
  };

  return (
    <>
      <Header name={currentApiKey?.id || ""} entityType="API Key" Icon={IconKey} divider />
      <Container size="sm" ml="0" pl="0">
        <form onSubmit={form.onSubmit(save)}>
          <Stack>
            <TextInput variant="filled" label="ID" value={currentApiKey?.id || ""} readOnly />
            <Textarea
              label="Description"
              autosize
              maxRows={10}
              key={form.key("name")}
              value={form.values.description}
              onChange={(event) => form.setFieldValue("description", event.currentTarget.value)}
            />
            <Select
              data-autofocus
              label="Access level"
              placeholder="Pick an access level"
              data={roleSelectData}
              value={form.values.role}
              error={form.errors.role}
              onChange={(value) => value && isValidKeyRole(value) && form.setFieldValue("role", value)}
              my="sm"
            />
            <Tooltip label="The password cannot be shown or changed. Please create a new API key if needed and possibly delete this one.">
              <TextInput label="Password" value="••••••••" readOnly variant="filled" />
            </Tooltip>
            <Group>
              <MaintainerButton
                leftSection={<IconTrash />}
                variant="outline"
                onClick={() => confirmDeleteApiKey(currentApiKey)}
                tooltip="Delete API key"
              >
                Delete
              </MaintainerButton>
              <MaintainerButton type="submit" disabled={!form.isDirty()} loading={form.submitting}>
                Save
              </MaintainerButton>
            </Group>
          </Stack>
        </form>
      </Container>
    </>
  );
}
