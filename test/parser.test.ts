import { parse, SseParser } from "../index.ts";

describe("SseParser", () => {
  test("parses chunked binary input", async () => {
    const parser = await SseParser.create();
    const encoder = new TextEncoder();

    expect(parser.feed(encoder.encode("event: update\ndata: hel"))).toEqual([]);

    const events = parser.feed(encoder.encode("lo\nid: 42\n\n"));

    expect(events).toEqual([
      {
        event: "update",
        data: "hello",
        id: "42",
      },
    ]);
    expect(parser.lastEventId).toBe("42");
  });

  test("tracks retry and flushes trailing events", async () => {
    const parser = await SseParser.create();

    expect(parser.feed("retry: 2500\n\n")).toEqual([]);
    expect(parser.retry).toBe(2500);

    expect(parser.feed("data: tail")).toEqual([]);
    expect(parser.finish()).toEqual([
      {
        event: "message",
        data: "tail",
      },
    ]);
  });

  test("parses a whole payload with helper", async () => {
    await expect(parse("data: one\ndata: two\n\n")).resolves.toEqual([
      {
        event: "message",
        data: "one\ntwo",
      },
    ]);
  });
});
