import { render, screen, fireEvent } from "@testing-library/react";
import { FrontmatterCard } from "./FrontmatterCard";

describe("FrontmatterCard", () => {
  it("빈 메타데이터면 아무것도 렌더링하지 않는다", () => {
    const { container } = render(
      <FrontmatterCard data={{}} onTagClick={() => {}} />
    );
    expect(container.firstChild).toBeNull();
  });

  it("title만 있으면 아무것도 렌더링하지 않는다", () => {
    const { container } = render(
      <FrontmatterCard data={{ title: "테스트 문서" }} onTagClick={() => {}} />
    );
    expect(container.firstChild).toBeNull();
  });

  it("tags 배열을 클릭 가능한 배지로 렌더링한다", () => {
    const onTagClick = vi.fn();
    render(
      <FrontmatterCard
        data={{ tags: ["react", "typescript"] }}
        onTagClick={onTagClick}
      />
    );
    expect(screen.getByText("#react")).toBeInTheDocument();
    expect(screen.getByText("#typescript")).toBeInTheDocument();
  });

  it("태그 배지 클릭 시 onTagClick이 해당 태그로 호출된다", () => {
    const onTagClick = vi.fn();
    render(
      <FrontmatterCard data={{ tags: ["react"] }} onTagClick={onTagClick} />
    );
    fireEvent.click(screen.getByText("#react"));
    expect(onTagClick).toHaveBeenCalledWith("react");
  });

  it("날짜 형식 값을 한국어로 포맷팅한다", () => {
    render(
      <FrontmatterCard
        data={{ created: "2026-03-31" }}
        onTagClick={() => {}}
      />
    );
    expect(screen.getByText("2026년 3월 31일")).toBeInTheDocument();
  });

  it("일반 문자열 값을 그대로 표시한다", () => {
    render(
      <FrontmatterCard data={{ author: "madup" }} onTagClick={() => {}} />
    );
    expect(screen.getByText("madup")).toBeInTheDocument();
    expect(screen.getByText("author")).toBeInTheDocument();
  });
});
