import { SavedPlayer } from "@/components/SavedPlayer";
import { keepPagesOnSite } from "@/lib/site-host";
export default async function SavedPage({params}:{params:Promise<{build:string}>}) {
  await keepPagesOnSite();
  return <SavedPlayer build={(await params).build}/>;
}
