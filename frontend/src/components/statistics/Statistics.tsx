import Quota from "./Quota.tsx";
import InfoAlert from "../InfoAlert.tsx";
import OrganizationHeader from "../organizations/OrganizationHeader.tsx";

export default function Statistics() {
  return (
    <>
      <OrganizationHeader />
      <InfoAlert stateName="statistics">
        This section shows your current usage and monthly sending quota. More detailed analytics and historical data
        will be added soon.
      </InfoAlert>
      <Quota />
    </>
  );
}
