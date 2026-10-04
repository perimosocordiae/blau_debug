#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = [
#     "flask",
# ]
# ///
import argparse
import json
import subprocess
import sys

from flask import Flask, jsonify, render_template, request


def get_app(args: argparse.Namespace) -> Flask:
    app = Flask(__name__)
    app.proc = subprocess.Popen(
        [args.game_path],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=sys.stderr,
        text=True,
    )

    @app.route("/")
    def home():
        return render_template("index.html", game_path=args.game_path)

    @app.route("/update", methods=["POST"])
    def update_data():
        try:
            data = request.get_json()
        except json.JSONDecodeError as e:
            return jsonify({"error": str(e)}), 400

        print('xxx', data)
        app.proc.stdin.write(json.dumps(data) + "\n")
        app.proc.stdin.flush()
        response = app.proc.stdout.readline()
        return jsonify(json.loads(response)), 200

    return app


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument(
        "--game_path", required=True, help="Path to the blau_debug game executable"
    )
    args = ap.parse_args()
    app = get_app(args)
    app.run(debug=True)
