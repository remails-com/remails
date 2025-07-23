import Quota from "./Quota.tsx";
import InfoAlert from "../InfoAlert.tsx";

export default function Statistics() {
  return (
    <>
      <InfoAlert stateName="statistics">
        This section shows your current usage and monthly sending quota. More detailed analytics and historical data
        will be added soon.
      </InfoAlert>
      <Quota />
    </>
  );
}
