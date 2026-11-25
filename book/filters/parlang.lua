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
--     unique across chapters.
-- Code text is never altered. The source path comes from the `book-source`
-- metadata set by book/build-book.

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

function Pandoc(doc)
  local meta_source = doc.meta["book-source"]
  if meta_source then
    source = pandoc.utils.stringify(meta_source)
    stem = source:gsub("^.*/", ""):gsub("%.md$", ""):gsub("[^%w]", "-")
  end
  doc.blocks = drop_toc(doc.blocks)
  return doc:walk({
    CodeBlock = code_block,
    Link = link,
    Header = header,
  })
end
