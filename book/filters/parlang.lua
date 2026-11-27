-- Pandoc Lua filter for the ParLang book, shared by the PDF (latex) and
-- EPUB (markdown intermediate, then epub3) builds.
--
-- Generic over every source file:
--   * a code block with an include="<repo-relative path>" attribute gets the
--     exact bytes of that file as its body (missing file = build error);
--   * latex: code blocks become fvextra verbatim environments (BookParCode for
--     `parlang`, BookCode otherwise); other formats keep them as code blocks;
--   * `mermaid` blocks become a "diagram omitted" note;
--   * links to local files or anchors become plain text, http(s)/mailto stay;
--   * "Table of Contents" sections are dropped;
--   * header identifiers are prefixed with the source file stem so they stay
--     unique across chapters;
--   * latex: inline code is split into several Code inlines, with
--     \allowbreak{} after runs of / . : , _ and a discouraged break
--     (\penalty50) inside CamelCase words;
--   * latex: a slash in prose allows a line break after it;
--   * latex: tables whose columns all have default widths get relative widths
--     proportional to their longest cell text, so pandoc emits p-columns.
-- Code block text is never altered; inline code keeps its text and only gains
-- break points. The source path comes from the `book-source` metadata set by
-- book/build-book.

local source = "book"
local stem = "doc"
local is_latex = FORMAT:match("latex") ~= nil

local function read_file(path)
  local handle = io.open(path, "rb")
  if not handle then
    error("parlang.lua: include file not found: " .. path)
  end
  local text = handle:read("a")
  handle:close()
  return (text:gsub("\n$", ""))
end

local function omitted_diagram()
  if is_latex then
    return pandoc.RawBlock("latex", "\\BookOmittedDiagram{" .. source .. "}")
  end
  local note = "Diagram omitted in this edition; see " .. source .. " in the repository."
  return pandoc.Para({ pandoc.Emph({ pandoc.Str(note) }) })
end

local function code_block(el)
  local include = el.attributes["include"]
  if include then
    el.text = read_file(include)
    el.attributes["include"] = nil
  end
  local lang = el.classes[1] or ""
  if lang == "mermaid" then
    return omitted_diagram()
  end
  if not is_latex then
    return el
  end
  local label = lang:gsub("[^%w%-]", "")
  local env = "BookCode"
  local opts = ""
  if label == "parlang" then
    env = "BookParCode"
  elseif label ~= "" then
    opts = "[label={" .. label .. "}]"
  end
  local latex = "\\begin{" .. env .. "}" .. opts .. "\n" .. el.text .. "\n\\end{" .. env .. "}"
  return pandoc.RawBlock("latex", latex)
end

local function link(el)
  local target = el.target
  if target:match("^https?://") or target:match("^mailto:") then
    return el
  end
  return el.content
end

local function header(el)
  if el.identifier ~= "" then
    el.identifier = stem .. "-" .. el.identifier
  end
  return el
end

local function is_toc_heading(block)
  if block.t ~= "Header" then
    return false
  end
  local title = pandoc.utils.stringify(block.content):lower()
  return title == "contents" or title:find("table of contents", 1, true) ~= nil
end

local function drop_toc(blocks)
  local out = pandoc.Blocks({})
  local skip_level = nil
  for _, block in ipairs(blocks) do
    if skip_level and block.t == "Header" and block.level <= skip_level then
      skip_level = nil
    end
    if skip_level == nil and is_toc_heading(block) then
      skip_level = block.level
    elseif skip_level == nil then
      out:insert(block)
    end
  end
  return out
end

-- latex: split inline code into several Code inlines so long paths and
-- identifiers can break at the line end: freely after runs of / . : , _
-- (\allowbreak{}), reluctantly inside CamelCase words (\penalty50).
-- Pandoc still escapes each piece and builds heading bookmarks from their
-- text.
local function breakable_code(el)
  local out = pandoc.Inlines({})
  local function piece(text, sep)
    if #out > 0 then
      out:insert(pandoc.RawInline("latex", sep))
    end
    out:insert(pandoc.Code(text, el.attr))
  end
  for run in el.text:gmatch("[^/.:,_]*[/.:,_]*") do
    if run ~= "" then
      local start = 1
      local sep = "\\allowbreak{}"
      for pos in run:gmatch("%l()%u") do
        piece(run:sub(start, pos - 1), sep)
        sep = "\\penalty50\\relax{}"
        start = pos
      end
      piece(run:sub(start), sep)
    end
  end
  if #out < 2 then
    return nil
  end
  return out
end

-- latex: a slash in prose (Church/Scott, or between code spans such as
-- Lam/Let/LetRec) allows a line break after it.
local function breakable_slash(el)
  if not el.text:find("/", 1, true) then
    return nil
  end
  local out = pandoc.Inlines({})
  for run in el.text:gmatch("[^/]*/*") do
    if run ~= "" then
      out:insert(pandoc.Str(run))
      if run:sub(-1) == "/" then
        out:insert(pandoc.RawInline("latex", "\\allowbreak{}"))
      end
    end
  end
  return out
end

local function default_width(width)
  return type(width) ~= "number" or width <= 0
end

local function cell_length(cell)
  local text = pandoc.utils.stringify(cell.contents)
  return utf8.len(text) or #text
end

local function measure_rows(rows, lengths)
  for _, row in ipairs(rows or {}) do
    for j, cell in ipairs(row.cells) do
      lengths[j] = math.max(lengths[j] or 0, cell_length(cell))
    end
  end
end

-- latex: tables whose columns all have default widths get relative widths
-- proportional to their longest cell text (clamped to 6..60 characters),
-- so pandoc emits wrapping p-columns instead of l-columns.
local function table_widths(tbl)
  for _, spec in ipairs(tbl.colspecs) do
    if not default_width(spec[2]) then
      return nil
    end
  end
  local lengths = {}
  measure_rows(tbl.head and tbl.head.rows, lengths)
  for _, body in ipairs(tbl.bodies or {}) do
    measure_rows(body.head, lengths)
    measure_rows(body.body, lengths)
  end
  measure_rows(tbl.foot and tbl.foot.rows, lengths)
  local total = 0
  local widths = {}
  for j = 1, #tbl.colspecs do
    widths[j] = math.min(math.max(lengths[j] or 0, 6), 60)
    total = total + widths[j]
  end
  if total == 0 then
    return nil
  end
  local specs = {}
  for j, spec in ipairs(tbl.colspecs) do
    specs[j] = { spec[1], widths[j] / total }
  end
  tbl.colspecs = specs
  return tbl
end

function Pandoc(doc)
  local meta_source = doc.meta["book-source"]
  if meta_source then
    source = pandoc.utils.stringify(meta_source)
    stem = source:gsub("^.*/", ""):gsub("%.md$", ""):gsub("[^%w]", "-")
  end
  doc.blocks = drop_toc(doc.blocks)
  local filter = {
    CodeBlock = code_block,
    Link = link,
    Header = header,
  }
  if is_latex then
    filter.Code = breakable_code
    filter.Str = breakable_slash
    filter.Table = table_widths
  end
  return doc:walk(filter)
end
