import { getCollection } from "astro:content";

export async function getNews() {
  return (await getCollection("news")).sort(
    (a, b) =>
      b.data.date.valueOf() - a.data.date.valueOf() || a.id.localeCompare(b.id),
  );
}

export const newsDate = (date: Date) => date.toISOString().slice(0, 10);
export const newsDateLabel = (date: Date) =>
  new Intl.DateTimeFormat("en", {
    year: "numeric",
    month: "long",
    day: "numeric",
    timeZone: "UTC",
  }).format(date);
