import { Card } from "../ui/Card";

function SectionBadge({ letter }: { letter: string }) {
  return (
    <span className="inline-flex items-center justify-center w-6 h-6 rounded-md text-xs font-bold bg-[var(--accent)] text-[var(--bg-primary)]">
      {letter}
    </span>
  );
}

function CodeBlock({ children }: { children: React.ReactNode }) {
  return (
    <div className="text-xs p-3 rounded-lg space-y-1 font-mono bg-[var(--bg-primary)] border border-[var(--border)]">
      {children}
    </div>
  );
}

function CodeInline({ children }: { children: React.ReactNode }) {
  return <code className="px-1 py-0.5 rounded text-xs bg-[var(--bg-tertiary)]">{children}</code>;
}

export function GuideView() {
  return (
    <div className="p-6 max-w-3xl mx-auto">
      <h2 className="text-lg font-bold text-[var(--accent)] mb-6">Obsidian Nexus 사용 가이드</h2>

      {/* Desktop App */}
      <div className="mb-8">
        <h3 className="text-base font-bold text-[var(--text-primary)] mb-3 flex items-center gap-2">
          <SectionBadge letter="D" /> 데스크톱 앱
        </h3>
        <div className="space-y-3">
          <Card>
            <h4 className="font-medium text-sm text-[var(--text-primary)] mb-2">볼트 추가</h4>
            <p className="text-sm text-[var(--text-tertiary)] mb-2">
              <strong className="text-[var(--text-secondary)]">프로젝트</strong> 탭에서 "볼트 폴더 추가" 버튼을 클릭하세요.
              Obsidian 볼트 폴더를 선택하면 자동으로 등록, 인덱싱, Obsidian 연동이 완료됩니다.
            </p>
            <div className="text-xs p-2 rounded-md bg-[var(--bg-primary)] border-l-2 border-[var(--accent)]">
              <strong>사전 준비:</strong> 등록하려는 폴더를 먼저 Obsidian 앱에서 한 번 열어 볼트로 초기화해야 합니다.
              Obsidian이 <CodeInline>.obsidian/</CodeInline> 폴더를 생성해야 Nexus가 볼트로 인식할 수 있습니다.
            </div>
          </Card>
          <Card>
            <h4 className="font-medium text-sm text-[var(--text-primary)] mb-2">문서 검색</h4>
            <p className="text-sm text-[var(--text-tertiary)]">
              <strong className="text-[var(--text-secondary)]">검색</strong> 탭에서 키워드를 입력하면 모든 볼트에서 문서를 찾습니다.
              키워드 / 벡터 / 하이브리드 검색 모드를 지원하며, 프로젝트와 태그 필터링이 가능합니다.
            </p>
          </Card>
          <Card>
            <h4 className="font-medium text-sm text-[var(--text-primary)] mb-2">문서 탐색 & 편집</h4>
            <p className="text-sm text-[var(--text-tertiary)]">
              검색 결과를 클릭하면 마크다운으로 렌더링된 내용을 확인할 수 있고,
              "Obsidian에서 열기" 버튼으로 Obsidian 앱에서 바로 편집할 수 있습니다.
            </p>
          </Card>
          <Card>
            <h4 className="font-medium text-sm text-[var(--text-primary)] mb-2">자동 설정</h4>
            <p className="text-sm text-[var(--text-tertiary)]">
              앱 실행 시 자동으로 MCP 서버를 AI 도구(Claude Desktop, Claude Code, Gemini CLI)에 등록하고,
              터미널에서 <CodeInline>nexus</CodeInline> 명령어를 바로 사용할 수 있도록 PATH에 추가합니다.
            </p>
          </Card>
        </div>
      </div>

      {/* CLI */}
      <div className="mb-8">
        <h3 className="text-base font-bold text-[var(--text-primary)] mb-3 flex items-center gap-2">
          <SectionBadge letter="C" /> CLI (Command Line Interface)
        </h3>
        <p className="text-sm text-[var(--text-tertiary)] mb-3">
          데스크톱 앱 설치 시 자동으로 포함됩니다. 터미널에서 <CodeInline>nexus</CodeInline> 명령어로 사용하세요.
        </p>
        <div className="space-y-3">
          <Card>
            <h4 className="font-medium text-sm text-[var(--text-primary)] mb-2">프로젝트 관리</h4>
            <CodeBlock>
              <p><span className="text-[var(--text-tertiary)]"># 볼트 등록</span></p>
              <p>nexus project add "My Vault" /path/to/vault</p>
              <p><span className="text-[var(--text-tertiary)]"># 프로젝트 목록</span></p>
              <p>nexus project list</p>
              <p><span className="text-[var(--text-tertiary)]"># 프로젝트 상세 정보</span></p>
              <p>nexus project info "project-id"</p>
            </CodeBlock>
          </Card>
          <Card>
            <h4 className="font-medium text-sm text-[var(--text-primary)] mb-2">검색</h4>
            <CodeBlock>
              <p><span className="text-[var(--text-tertiary)]"># 키워드 검색 (기본)</span></p>
              <p>nexus search "검색어"</p>
              <p><span className="text-[var(--text-tertiary)]"># 특정 프로젝트에서 벡터 검색</span></p>
              <p>nexus search "의미 검색" --project "My Vault" --mode vector</p>
              <p><span className="text-[var(--text-tertiary)]"># 하이브리드 검색</span></p>
              <p>nexus search "질문" --mode hybrid --limit 10</p>
            </CodeBlock>
          </Card>
          <Card>
            <h4 className="font-medium text-sm text-[var(--text-primary)] mb-2">문서 접근</h4>
            <CodeBlock>
              <p><span className="text-[var(--text-tertiary)]"># 문서 내용 가져오기</span></p>
              <p>nexus doc get "project-id" "notes/file.md"</p>
              <p><span className="text-[var(--text-tertiary)]"># 문서 메타데이터</span></p>
              <p>nexus doc meta "project-id" "notes/file.md"</p>
              <p><span className="text-[var(--text-tertiary)]"># 프로젝트 문서 목록</span></p>
              <p>nexus doc list "project-id"</p>
            </CodeBlock>
          </Card>
          <Card>
            <h4 className="font-medium text-sm text-[var(--text-primary)] mb-2">인덱싱 & 자동 감시</h4>
            <CodeBlock>
              <p><span className="text-[var(--text-tertiary)]"># 수동 인덱싱 (변경분만)</span></p>
              <p>nexus index "project-id"</p>
              <p><span className="text-[var(--text-tertiary)]"># 실시간 파일 감시 + 자동 인덱싱</span></p>
              <p>nexus watch "project-id"</p>
            </CodeBlock>
          </Card>
          <Card>
            <h4 className="font-medium text-sm text-[var(--text-primary)] mb-2">프로젝트 온보딩</h4>
            <CodeBlock>
              <p><span className="text-[var(--text-tertiary)]"># 현재 프로젝트에 MCP + librarian 스킬 자동 설정</span></p>
              <p>nexus onboard</p>
              <p><span className="text-[var(--text-tertiary)]"># 특정 경로 지정</span></p>
              <p>nexus onboard /path/to/project</p>
            </CodeBlock>
            <p className="text-xs text-[var(--text-tertiary)] mt-2">.mcp.json, .claude/agents, .claude/skills 파일이 자동 생성됩니다.</p>
          </Card>
        </div>
      </div>

      {/* MCP Server */}
      <div className="mb-8">
        <h3 className="text-base font-bold text-[var(--text-primary)] mb-3 flex items-center gap-2">
          <SectionBadge letter="M" /> MCP 서버 (AI 에이전트 연동)
        </h3>
        <p className="text-sm text-[var(--text-tertiary)] mb-3">
          앱 실행 시 자동으로 AI 도구에 등록됩니다. Claude Code, Claude Desktop, Gemini CLI에서 자동 사용 가능합니다.
        </p>
        <div className="space-y-3">
          <Card>
            <h4 className="font-medium text-sm text-[var(--text-primary)] mb-2">검색 도구</h4>
            <div className="text-xs space-y-2">
              <div className="flex gap-2"><CodeInline>nexus_search</CodeInline><span className="text-[var(--text-tertiary)]">하이브리드/키워드/벡터 검색, 태그 필터, 인기도 부스트</span></div>
              <div className="flex gap-2"><CodeInline>nexus_resolve_alias</CodeInline><span className="text-[var(--text-tertiary)]">별칭으로 문서 찾기</span></div>
            </div>
          </Card>
          <Card>
            <h4 className="font-medium text-sm text-[var(--text-primary)] mb-2">문서 접근 도구</h4>
            <div className="text-xs space-y-2">
              <div className="flex gap-2"><CodeInline>nexus_get_document</CodeInline><span className="text-[var(--text-tertiary)]">문서 전체 내용 가져오기</span></div>
              <div className="flex gap-2"><CodeInline>nexus_get_section</CodeInline><span className="text-[var(--text-tertiary)]">특정 섹션만 가져오기 (토큰 절약!)</span></div>
              <div className="flex gap-2"><CodeInline>nexus_get_metadata</CodeInline><span className="text-[var(--text-tertiary)]">프론트매터, 태그, 인덱싱 상태</span></div>
            </div>
          </Card>
          <Card>
            <h4 className="font-medium text-sm text-[var(--text-primary)] mb-2">그래프 탐색 도구</h4>
            <div className="text-xs space-y-2">
              <div className="flex gap-2"><CodeInline>nexus_get_backlinks</CodeInline><span className="text-[var(--text-tertiary)]">이 문서를 링크하는 문서들</span></div>
              <div className="flex gap-2"><CodeInline>nexus_get_links</CodeInline><span className="text-[var(--text-tertiary)]">이 문서가 링크하는 문서들</span></div>
            </div>
          </Card>
          <Card>
            <h4 className="font-medium text-sm text-[var(--text-primary)] mb-2">관리 도구</h4>
            <div className="text-xs space-y-2">
              <div className="flex gap-2"><CodeInline>nexus_list_projects</CodeInline><span className="text-[var(--text-tertiary)]">등록된 볼트 목록</span></div>
              <div className="flex gap-2"><CodeInline>nexus_list_documents</CodeInline><span className="text-[var(--text-tertiary)]">프로젝트 문서 목록 (태그 필터)</span></div>
              <div className="flex gap-2"><CodeInline>nexus_index_project</CodeInline><span className="text-[var(--text-tertiary)]">증분 / 전체 리인덱스</span></div>
              <div className="flex gap-2"><CodeInline>nexus_status</CodeInline><span className="text-[var(--text-tertiary)]">시스템 상태 확인 (Ollama, DB)</span></div>
              <div className="flex gap-2"><CodeInline>nexus_onboard</CodeInline><span className="text-[var(--text-tertiary)]">프로젝트에 librarian 스킬 자동 설정</span></div>
            </div>
          </Card>
        </div>
      </div>

      {/* Recommended workflow */}
      <Card className="border-[var(--accent)]/20">
        <h3 className="font-medium text-sm text-[var(--accent)] mb-2">추천 워크플로우</h3>
        <div className="text-sm text-[var(--text-tertiary)] space-y-1">
          <p>1. 데스크톱 앱으로 Obsidian 볼트 등록 & 인덱싱</p>
          <p>2. <CodeInline>nexus onboard</CodeInline>로 프로젝트에 MCP 연동 설정</p>
          <p>3. AI 에이전트가 <CodeInline>nexus_search</CodeInline> → <CodeInline>nexus_get_section</CodeInline>으로 필요한 문서만 가져옴</p>
          <p>4. <CodeInline>nexus_get_backlinks</CodeInline>로 관련 문서 탐색</p>
        </div>
      </Card>
    </div>
  );
}
