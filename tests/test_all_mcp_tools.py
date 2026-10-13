#!/usr/bin/env python3
import json
import urllib.request
import sys

BASE_URL = "http://localhost:3000/v1/mcp"
API_KEY = "Yk8gJ79bYxfFAGmwSS4panA52E5cG83LxF7aLMpl"

class McpClient:
    def __init__(self, base_url, api_key):
        self.base_url = base_url
        self.api_key = api_key
        self.session_id = None
        self.req_id = 0

    def _send(self, payload):
        self.req_id += 1
        payload["id"] = self.req_id
        data = json.dumps(payload).encode("utf-8")
        headers = {
            "Content-Type": "application/json",
            "Accept": "application/json, text/event-stream",
            "X-Api-Key": self.api_key,
        }
        if self.session_id:
            headers["mcp-session-id"] = self.session_id

        req = urllib.request.Request(self.base_url, data=data, headers=headers)
        with urllib.request.urlopen(req) as resp:
            if not self.session_id:
                self.session_id = resp.headers.get("mcp-session-id")
            body = resp.read().decode("utf-8")

        for line in body.splitlines():
            if line.startswith("data: {"):
                return json.loads(line[6:])
        return None

    def initialize(self):
        res = self._send({
            "jsonrpc": "2.0",
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "test-runner", "version": "1.0.0"}
            }
        })
        return res

    def list_tools(self):
        return self._send({
            "jsonrpc": "2.0",
            "method": "tools/list"
        })

    def call_tool(self, name, args):
        res = self._send({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": name,
                "arguments": args
            }
        })
        return res

def parse_tool_result(res):
    if not res:
        return False, "No response"
    if "error" in res:
        return False, f"RPC Error: {res['error']}"
    result = res.get("result", {})
    if result.get("isError"):
        content = result.get("content", [])
        text = content[0].get("text") if content else "Unknown error"
        return False, f"Tool Error: {text}"
    content = result.get("content", [])
    text = content[0].get("text") if content else ""
    try:
        data = json.loads(text)
        return True, data
    except Exception:
        return True, text

def run_tests():
    client = McpClient(BASE_URL, API_KEY)
    print("Initializing MCP connection...")
    init_res = client.initialize()
    print("Session ID:", client.session_id)

    tools_res = client.list_tools()
    all_tools = [t["name"] for t in tools_res["result"]["tools"]]
    print(f"Discovered {len(all_tools)} tools.")

    tested_tools = set()
    failed_tools = set()

    def test(name, args):
        tested_tools.add(name)
        print(f"\n[TEST] {name} with args {args}...")
        res = client.call_tool(name, args)
        ok, data = parse_tool_result(res)
        if ok:
            print(f"  ✓ SUCCESS: {str(data)[:120]}")
            return data
        else:
            print(f"  ✗ FAILED: {data}")
            failed_tools.add(name)
            return None

    # 1. Brands
    created_brand = test("create_brand", {
        "name": "MCP Test Brand",
        "from_name": "MCP Tester",
        "from_email": "mcp-test@example.com",
        "reply_to": "reply@example.com"
    })
    
    brand_id = created_brand.get("id") if isinstance(created_brand, dict) else None
    print(f"--> Created Brand ID: {brand_id}")

    test("list_brands", {})
    if brand_id:
        test("get_brand", {"brand_id": brand_id})
        test("update_brand", {"brand_id": brand_id, "name": "MCP Test Brand Updated"})

    # 2. Lists
    list_id = None
    if brand_id:
        created_list = test("create_list", {
            "brand_id": brand_id,
            "name": "MCP Test List"
        })
        list_id = created_list.get("id") if isinstance(created_list, dict) else None
        print(f"--> Created List ID: {list_id}")

        test("list_lists_for_brand", {"brand_id": brand_id})

    if list_id:
        test("get_list", {"list_id": list_id})
        test("update_list", {"list_id": list_id, "name": "MCP Test List Updated"})

    # 3. Subscribers
    if list_id:
        test("subscribe", {
            "list_id": list_id,
            "email": "sub1@example.com",
            "name": "Subscriber One"
        })
        test("subscribe", {
            "list_id": list_id,
            "email": "sub2@example.com",
            "name": "Subscriber Two"
        })
        test("get_subscriber_status", {
            "list_id": list_id,
            "email": "sub1@example.com"
        })
        test("get_subscriber_count", {
            "list_id": list_id
        })
        test("query_subscribers", {
            "list_id": 1
        })
        test("unsubscribe", {
            "list_id": list_id,
            "email": "sub1@example.com"
        })
        test("delete_subscriber", {
            "list_id": list_id,
            "email": "sub2@example.com"
        })

    # 4. Templates
    template_id = None
    if brand_id:
        created_tmpl = test("create_template", {
            "brand_id": brand_id,
            "name": "MCP Template",
            "html_text": "<h1>Hello from MCP</h1>"
        })
        template_id = created_tmpl.get("id") if isinstance(created_tmpl, dict) else None
        print(f"--> Created Template ID: {template_id}")

        test("list_templates", {"brand_id": brand_id})

    if template_id:
        test("update_template", {
            "template_id": template_id,
            "name": "MCP Template Updated"
        })

    # 5. Campaigns
    campaign_id = None
    if brand_id and list_id:
        created_camp = test("create_campaign", {
            "brand_id": brand_id,
            "list_ids": [list_id],
            "title": "MCP Campaign Title",
            "from_name": "MCP Campaigner",
            "from_email": "camp@example.com",
            "reply_to": "reply@example.com",
            "subject": "MCP Subject",
            "html_text": "<p>Hello Campaign</p>"
        })
        if isinstance(created_camp, dict):
            campaign_id = created_camp.get("campaign_id")
        print(f"--> Created Campaign ID: {campaign_id}")

        test("list_campaigns", {})

    if campaign_id:
        test("get_campaign_report", {"campaign_id": campaign_id})

    # 6. Segments
    segment_id = None
    if brand_id and list_id:
        created_seg = test("create_segment", {
            "brand_id": brand_id,
            "list_id": list_id,
            "name": "MCP Segment"
        })
        segment_id = created_seg.get("id") if isinstance(created_seg, dict) else None
        print(f"--> Created Segment ID: {segment_id}")

        test("list_segments", {"brand_id": brand_id})

    # 7. Rules
    rule_id = None
    if brand_id:
        created_rule = test("create_rule", {
            "brand_id": brand_id,
            "trigger": "subscribe",
            "action": "webhook",
            "webhook_url": "https://example.com/webhook"
        })
        rule_id = created_rule.get("id") if isinstance(created_rule, dict) else None
        print(f"--> Created Rule ID: {rule_id}")

        test("list_rules", {"brand_id": brand_id})

    if rule_id:
        test("toggle_rule", {"rule_id": rule_id})

    # 8. Settings - Suppression
    supp_id = None
    if brand_id:
        test("add_suppression", {
            "brand_id": brand_id,
            "email": "suppressed@example.com"
        })
        supp_list = test("list_suppression", {"brand_id": brand_id})
        if isinstance(supp_list, list) and len(supp_list) > 0:
            supp_id = supp_list[0].get("id")
        if supp_id:
            test("delete_suppression", {"id": supp_id})

    # 8. Settings - Blocked Domains
    dom_id = None
    if brand_id:
        test("add_blocked_domain", {
            "brand_id": brand_id,
            "domain": "spam-domain.com"
        })
        dom_list = test("list_blocked_domains", {"brand_id": brand_id})
        if isinstance(dom_list, list) and len(dom_list) > 0:
            dom_id = dom_list[0].get("id")
        if dom_id:
            test("delete_blocked_domain", {"id": dom_id})

    # Teardown / Deletes
    if rule_id:
        test("delete_rule", {"rule_id": rule_id})

    if segment_id:
        test("delete_segment", {"segment_id": segment_id})

    if campaign_id:
        test("delete_campaign", {"campaign_id": campaign_id})

    if template_id:
        test("delete_template", {"template_id": template_id})

    if list_id:
        test("delete_list", {"list_id": list_id})

    if brand_id:
        test("delete_brand", {"brand_id": brand_id})

    print("\n" + "="*50)
    print("SUMMARY")
    print("="*50)
    print(f"Total Tools Discovered: {len(all_tools)}")
    print(f"Total Tools Tested:     {len(tested_tools)}")
    untested = set(all_tools) - tested_tools
    print(f"Untested Tools:         {len(untested)} {untested if untested else ''}")
    print(f"Failed Tools:           {len(failed_tools)} {failed_tools if failed_tools else ''}")
    
    if len(failed_tools) == 0 and len(untested) == 0:
        print("\n ALL 37 MCP TOOLS PASSED SUCCESSFULLY! ")
        sys.exit(0)
    else:
        print("\n SOME MCP TOOLS FAILED OR WERE NOT TESTED.")
        sys.exit(1)

if __name__ == "__main__":
    run_tests()
